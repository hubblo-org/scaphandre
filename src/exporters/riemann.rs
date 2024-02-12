//! # RiemannExporter
//!
//! The Riemann exporter sends metrics to a [Riemann](https://riemann.io/) server.

use crate::exporters::utils::get_hostname;
use crate::exporters::*;
use crate::sensors::Sensor;
use chrono::Utc;
use riemann_client::proto::{Attribute, Event};
use riemann_client::Client;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Riemann server default ipv4/ipv6 address
const DEFAULT_IP_ADDRESS: &str = "localhost";

/// Riemann server default port
const DEFAULT_PORT: u16 = 5555;

/// RiemannClient is a simple client implementation on top of the
/// [rust-riemann_client](https://github.com/borntyping/rust-riemann_client) library.
///
/// It allows to connect to a Riemann server and send metrics.
struct RiemannClient {
    client: Client,
}

impl RiemannClient {
    /// Send metrics to the server.
    fn send_metric(&mut self, metric: &Metric) {
        let mut event = Event::new();

        let mut attributes: Vec<Attribute> = vec![];
        for (key, value) in &metric.attributes {
            let mut attribute = Attribute::new();
            attribute.set_key(key.clone());
            attribute.set_value(value.clone());
            attributes.push(attribute);
        }

        event.set_time(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        );
        event.set_ttl(metric.ttl);
        event.set_host(metric.hostname.to_string());
        event.set_service(metric.name.to_string());
        event.set_state(metric.state.to_string());
        event.set_tags(protobuf::RepeatedField::from_vec(metric.tags.clone()));
        if !attributes.is_empty() {
            event.set_attributes(protobuf::RepeatedField::from_vec(attributes));
        }
        event.set_description(metric.description.to_string());

        match metric.metric_value {
            // MetricValueType::IntSigned(value) => event.set_metric_sint64(value),
            // MetricValueType::Float(value) => event.set_metric_f(value),
            //MetricValueType::FloatDouble(value) => event.set_metric_d(value),
            MetricValueType::IntUnsigned(value) => event.set_metric_sint64(
                i64::try_from(value).expect("Metric cannot be converted to signed integer."),
            ),
            MetricValueType::Text(ref value) => {
                let value = value.replace(',', ".").replace('\n', "");
                if value.contains('.') {
                    event.set_metric_d(value.parse::<f64>().expect("Cannot parse metric value."));
                } else {
                    event.set_metric_sint64(
                        value.parse::<i64>().expect("Cannot parse metric value."),
                    );
                }
            }
        }

        self.client
            .event(event)
            .expect("Fail to send metric to Riemann");
    }
}

/// An exporter that sends metrics to a Riemann server.
pub struct RiemannExporter {
    metric_generator: MetricGenerator,
    riemann_client: RiemannClient,
    args: ExporterArgs,
}

/// Contains the options of the Riemann exporter.
#[derive(clap::Args, Debug)]
pub struct ExporterArgs {
    /// Address of the Riemann server. If mTLS is used this must be the server's FQDN.
    #[arg(short, long, default_value = DEFAULT_IP_ADDRESS)]
    pub address: String,

    /// TCP port number of the Riemann server
    #[arg(short, long, default_value_t = DEFAULT_PORT)]
    pub port: u16,

    /// Duration between each metric dispatch, in seconds
    #[arg(short, long, default_value_t = 5)]
    pub dispatch_interval: u64,

    /// Apply labels to metrics of processes looking like a Qemu/KVM virtual machine
    #[arg(short, long)]
    pub qemu: bool,

    /// Monitor and apply labels for processes running as containers
    #[arg(long)]
    pub containers: bool,

    /// Connect to Riemann using mTLS instead of plain TCP.
    #[arg(
        long,
        requires = "address",
        requires = "ca_file",
        requires = "cert_file",
        requires = "key_file"
    )]
    pub mtls: bool,

    /// CA certificate file (.pem format)
    #[arg(long = "ca", requires = "mtls")]
    pub ca_file: Option<String>,

    /// Client certificate file (.pem format)
    #[arg(long = "cert", requires = "mtls")]
    pub cert_file: Option<String>,

    /// Client RSA key file
    #[arg(long = "key", requires = "mtls")]
    pub key_file: Option<String>,
}

impl RiemannExporter {
    /// Returns a RiemannExporter instance.
    pub fn new(sensor: &dyn Sensor, args: ExporterArgs) -> RiemannExporter {
        // Prepare the retrieval of the measurements
        let topo = sensor
            .get_topology()
            .expect("sensor topology should be available");
        let metric_generator =
            MetricGenerator::new(topo, utils::get_hostname(), args.qemu, args.containers);

        // Initialize the connection to the Riemann server
        let client = if args.mtls {
            Client::connect_tls(
                &args.address,
                args.port,
                &args.ca_file.clone().unwrap(),
                &args.cert_file.clone().unwrap(),
                &args.key_file.clone().unwrap(),
            )
            .expect("failed to connect to Riemann using mTLS")
        } else {
            Client::connect(&(args.address.clone(), args.port))
                .expect("failed to connect to Riemann using raw TCP")
        };
        let riemann_client = RiemannClient { client };
        RiemannExporter {
            metric_generator,
            riemann_client,
            args,
        }
    }
}

impl Exporter for RiemannExporter {
    /// Entry point of the RiemannExporter.
    fn run(&mut self) {
        info!(
            "{}: Starting Riemann exporter",
            Utc::now().format("%Y-%m-%dT%H:%M:%S")
        );
        println!("Press CTRL-C to stop scaphandre");

        let dispatch_interval = Duration::from_secs(self.args.dispatch_interval);
        println!("Dispatch interval is {dispatch_interval:?}");

        loop {
            info!(
                "{}: Beginning of measure loop",
                Utc::now().format("%Y-%m-%dT%H:%M:%S")
            );

            self.metric_generator
                .topology
                .proc_tracker
                .clean_terminated_process_records_vectors();

            info!(
                "{}: Refresh topology",
                Utc::now().format("%Y-%m-%dT%H:%M:%S")
            );
            self.metric_generator.topology.refresh();

            info!("{}: Refresh data", Utc::now().format("%Y-%m-%dT%H:%M:%S"));
            // Here we need a specific behavior for process metrics, so we call each gen function
            // and then implement that specific behavior (we don't use gen_all_metrics).
            self.metric_generator.gen_self_metrics();
            self.metric_generator.gen_host_metrics();
            self.metric_generator.gen_socket_metrics();

            let mut data = vec![];
            let processes_tracker = &self.metric_generator.topology.proc_tracker;

            for pid in processes_tracker.get_alive_pids() {
                let exe = processes_tracker.get_process_name(pid);
                let cmdline = processes_tracker.get_process_cmdline(pid);

                let mut attributes = HashMap::new();
                attributes.insert("pid".to_string(), pid.to_string());

                attributes.insert("exe".to_string(), exe.clone());

                if let Some(cmdline_str) = cmdline {
                    attributes.insert("cmdline".to_string(), cmdline_str.replace('\"', "\\\""));

                    if self.args.qemu {
                        if let Some(vmname) = utils::filter_qemu_cmdline(&cmdline_str) {
                            attributes.insert("vmname".to_string(), vmname);
                        }
                    }
                }

                // Here we define a metric name with pid + exe string suffix as riemann needs
                // to differentiate services/metrics
                let metric_name = format!(
                    "{}_{}_{}",
                    "scaph_process_power_consumption_microwatts", pid, exe
                );
                if let Some(power) = self
                    .metric_generator
                    .topology
                    .get_process_power_consumption_microwatts(pid)
                {
                    data.push(Metric {
                        name: metric_name,
                        metric_type: String::from("gauge"),
                        ttl: 60.0,
                        hostname: get_hostname(),
                        timestamp: power.timestamp,
                        state: String::from("ok"),
                        tags: vec!["scaphandre".to_string()],
                        attributes,
                        description: String::from("Power consumption due to the process, measured on at the topology level, in microwatts"),
                        metric_value: MetricValueType::Text(power.value),
                    });
                }
            }
            // Send all data
            info!("{}: Send data", Utc::now().format("%Y-%m-%dT%H:%M:%S"));
            for metric in self.metric_generator.pop_metrics() {
                self.riemann_client.send_metric(&metric);
            }
            for metric in data {
                self.riemann_client.send_metric(&metric);
            }

            // Pause for some time
            std::thread::sleep(dispatch_interval);
        }
    }

    fn kind(&self) -> &str {
        "riemann"
    }
}

//  Copyright 2020 The scaphandre authors.
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
