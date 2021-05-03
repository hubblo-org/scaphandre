//! # RiemannExporter
//!
//! `RiemannExporter` implementation, sends metrics to a [Riemann](https://riemann.io/)
//! server.
use crate::exporters::utils::get_hostname;
use crate::exporters::*;
use crate::sensors::Sensor;
use chrono::Utc;
use clap::Arg;
use riemann_client::proto::Attribute;
use riemann_client::proto::Event;
use riemann_client::Client;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Riemann server default ipv4/ipv6 address
const DEFAULT_IP_ADDRESS: &str = "localhost";

/// Riemann server default port
const DEFAULT_PORT: &str = "5555";

/// RiemannClient is a simple client implementation on top of the
/// [rust-riemann_client](https://github.com/borntyping/rust-riemann_client) library.
///
/// It allows to connect to a Riemann server and send metrics.
struct RiemannClient {
    client: Client,
}

impl RiemannClient {
    /// Instanciate the Riemann client either with mTLS or using raw TCP.
    fn new(parameters: &ArgMatches) -> RiemannClient {
        let address = String::from(parameters.value_of("address").unwrap());
        let port = parameters
            .value_of("port")
            .unwrap()
            .parse::<u16>()
            .expect("Fail parsing port number");
        let client: Client;
        if parameters.is_present("mtls") {
            let cafile = parameters.value_of("cafile").unwrap();
            let certfile = parameters.value_of("certfile").unwrap();
            let keyfile = parameters.value_of("keyfile").unwrap();
            client = Client::connect_tls(&address, port, cafile, certfile, keyfile)
                .expect("Fail to connect to Riemann server using mTLS");
        } else {
            client = Client::connect(&(address, port))
                .expect("Fail to connect to Riemann server using raw TCP");
        }
        RiemannClient { client }
    }

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
            MetricValueType::FloatDouble(value) => event.set_metric_d(value),
            MetricValueType::IntUnsigned(value) => event.set_metric_sint64(
                i64::try_from(value).expect("Metric cannot be converted to signed integer."),
            ),
            MetricValueType::Text(ref value) => {
                let value = value.replace(",", ".").replace("\n", "");
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

/// Exporter sends metrics to a Riemann server.
pub struct RiemannExporter {
    /// Sensor instance that is used to generate the Topology and
    /// thus get power consumption metrics.
    sensor: Box<dyn Sensor>,
}

impl RiemannExporter {
    /// Returns a RiemannExporter instance.
    pub fn new(sensor: Box<dyn Sensor>) -> RiemannExporter {
        RiemannExporter { sensor }
    }
}

impl Exporter for RiemannExporter {
    /// Entry point of the RiemannExporter.
    fn run(&mut self, parameters: ArgMatches) {
        let dispatch_duration: u64 = parameters
            .value_of("dispatch_duration")
            .unwrap()
            .parse()
            .expect("Wrong dispatch_duration value, should be a number of seconds");

        let hostname = get_hostname();

        let mut rclient = RiemannClient::new(&parameters);

        info!(
            "{}: Starting Riemann exporter",
            Utc::now().format("%Y-%m-%dT%H:%M:%S")
        );
        println!("Press CTRL-C to stop scaphandre");
        println!("Measurement step is: {}s", dispatch_duration);

        let mut topology = self.sensor.get_topology().unwrap();
        loop {
            info!(
                "{}: Beginning of measure loop",
                Utc::now().format("%Y-%m-%dT%H:%M:%S")
            );

            topology
                .proc_tracker
                .clean_terminated_process_records_vectors();

            info!(
                "{}: Refresh topology",
                Utc::now().format("%Y-%m-%dT%H:%M:%S")
            );
            topology.refresh();

            info!("{}: Refresh data", Utc::now().format("%Y-%m-%dT%H:%M:%S"));
            let mut metric_generator = MetricGenerator::new(&topology, &hostname);
            // Here we need a specific behavior for process metrics, so we call each gen function
            // and then implement that specific behavior (we don't use gen_all_metrics).
            metric_generator.gen_self_metrics();
            metric_generator.gen_host_metrics();
            metric_generator.gen_socket_metrics();

            let mut data = vec![];
            let processes_tracker = &metric_generator.topology.proc_tracker;

            for pid in processes_tracker.get_alive_pids() {
                let exe = processes_tracker.get_process_name(pid);
                let cmdline = processes_tracker.get_process_cmdline(pid);

                let mut attributes = HashMap::new();
                attributes.insert("pid".to_string(), pid.to_string());

                attributes.insert("exe".to_string(), exe.clone());

                if let Some(cmdline_str) = cmdline {
                    attributes.insert("cmdline".to_string(), cmdline_str.replace("\"", "\\\""));

                    if parameters.is_present("qemu") {
                        if let Some(vmname) = utils::filter_qemu_cmdline(&cmdline_str) {
                            attributes.insert("vmname".to_string(), vmname);
                        }
                    }
                }

                // Here we define a metric name with pid + exe string suffix as riemann needs
                // to differentiate services/metrics
                let metric_name = format!(
                    "{}_{}_{}",
                    "scaph_process_power_consumption_microwatts",
                    pid.to_string(),
                    exe
                );
                if let Some(power) = topology.get_process_power_consumption_microwatts(pid) {
                    data.push(Metric {
                        name: metric_name,
                        metric_type: String::from("gauge"),
                        ttl: 60.0,
                        hostname: get_hostname(),
                        state: String::from("ok"),
                        tags: vec!["scaphandre".to_string()],
                        attributes,
                        description: String::from("Power consumption due to the process, measured on at the topology level, in microwatts"),
                        metric_value: MetricValueType::Text(power.to_string()),
                    });
                }
            }
            // Send all data
            info!("{}: Send data", Utc::now().format("%Y-%m-%dT%H:%M:%S"));
            for metric in metric_generator.get_metrics() {
                rclient.send_metric(metric);
            }
            for metric in data {
                rclient.send_metric(&metric);
            }

            thread::sleep(Duration::new(dispatch_duration, 0));
        }
    }

    /// Returns options understood by the exporter.
    fn get_options() -> Vec<clap::Arg<'static, 'static>> {
        let mut options = Vec::new();
        let arg = Arg::with_name("address")
            .default_value(DEFAULT_IP_ADDRESS)
            .help("Riemann ipv6 or ipv4 address. If mTLS is used then server fqdn must be provided")
            .long("address")
            .short("a")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("port")
            .default_value(DEFAULT_PORT)
            .help("Riemann TCP port number")
            .long("port")
            .short("p")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("dispatch_duration")
            .default_value("5")
            .help("Duration between metrics dispatch")
            .long("dispatch")
            .short("d")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("qemu")
            .help("Instruct that scaphandre is running on an hypervisor")
            .long("qemu")
            .short("q")
            .required(false)
            .takes_value(false);
        options.push(arg);

        let arg = Arg::with_name("mtls")
            .help("Connect to a Riemann server using mTLS. Parameters address, ca, cert and key must be defined.")
            .long("mtls")
            .required(false)
            .takes_value(false)
            .requires_all(&["address","cafile", "certfile", "keyfile"]);
        options.push(arg);

        let arg = Arg::with_name("cafile")
            .help("CA certificate file (.pem format)")
            .long("ca")
            .required(false)
            .takes_value(true)
            .display_order(1000)
            .requires("mtls");
        options.push(arg);

        let arg = Arg::with_name("certfile")
            .help("Client certificate file (.pem format)")
            .long("cert")
            .required(false)
            .takes_value(true)
            .display_order(1001)
            .requires("mtls");
        options.push(arg);

        let arg = Arg::with_name("keyfile")
            .help("Client RSA key")
            .long("key")
            .required(false)
            .takes_value(true)
            .display_order(1001)
            .requires("mtls");
        options.push(arg);

        options
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
