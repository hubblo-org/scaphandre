use crate::exporters::*;
use crate::sensors::{RecordGenerator, Sensor};
use chrono::Utc;
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

/// Riemann client
struct Riemann {
    client: Client,
}

impl Riemann {
    fn new(address: &str, port: &str) -> Riemann {
        let address = String::from(address);
        let port = port.parse::<u16>().expect("Fail parsing port number");
        let client = Client::connect(&(address, port)).expect("Fail to connect to Riemann server");
        Riemann { client }
    }

    fn send_metric(&mut self, msg: &Metric) {
        let mut event = Event::new();

        let mut attributes: Vec<Attribute> = vec![];
        for (key, value) in &msg.attributes {
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
        event.set_ttl(msg.ttl);
        event.set_host(msg.hostname.to_string());
        event.set_service(msg.name.to_string());
        event.set_state(msg.state.to_string());
        event.set_tags(protobuf::RepeatedField::from_vec(msg.tags.clone()));
        if !attributes.is_empty() {
            event.set_attributes(protobuf::RepeatedField::from_vec(attributes));
        }
        event.set_description(msg.description.to_string());

        match msg.metric_value {
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

/// Exporter sends metrics to a Riemann server
pub struct RiemannExporter {
    /// Sensor instance that is used to generate the Topology and
    /// thus get power consumption metrics.
    sensor: Box<dyn Sensor>,
}

impl RiemannExporter {
    /// Instantiates RiemannExporter and returns the instance.
    pub fn new(sensor: Box<dyn Sensor>) -> RiemannExporter {
        RiemannExporter { sensor }
    }
}

impl Exporter for RiemannExporter {
    fn run(&mut self, parameters: ArgMatches) {
        let dispatch_duration: u64 = parameters
            .value_of("dispatch_duration")
            .unwrap()
            .parse()
            .expect("Wrong dispatch_duration value, should be a number of seconds");

        let hostname = String::from(
            hostname::get()
                .expect("Fail to get system hostname")
                .to_str()
                .unwrap(),
        );

        let mut rclient = Riemann::new(
            parameters.value_of("address").unwrap(),
            parameters.value_of("port").unwrap(),
        );

        info!(
            "{}: Starting Riemann exporter",
            Utc::now().format("%Y-%m-%dT%H:%M:%S")
        );
        println!("Press CTRL-C to stop scaphandre");
        println!("Measurement step is: {}s", dispatch_duration);

        let mut topology = self.sensor.get_topology().unwrap();
        let metric_generator = MetricGenerator;
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
            let mut data: Vec<Metric> = Vec::new();
            let records = topology.get_records_passive();

            info!(
                "{}: Get self metrics",
                Utc::now().format("%Y-%m-%dT%H:%M:%S")
            );
            metric_generator.get_self_metrics(&topology, &mut data, &hostname);
            info!(
                "{}: Get host metrics",
                Utc::now().format("%Y-%m-%dT%H:%M:%S")
            );
            metric_generator.get_host_metrics(&topology, &mut data, &hostname, &records);
            info!(
                "{}: Get socket metrics",
                Utc::now().format("%Y-%m-%dT%H:%M:%S")
            );
            metric_generator.get_socket_metrics(&topology, &mut data, &hostname);
            info!(
                "{}: Get system metrics",
                Utc::now().format("%Y-%m-%dT%H:%M:%S")
            );
            metric_generator.get_system_metrics(&topology, &mut data, &hostname);
            info!(
                "{}: Get process metrics",
                Utc::now().format("%Y-%m-%dT%H:%M:%S")
            );
            metric_generator.get_process_metrics(
                &topology,
                &mut data,
                &hostname,
                parameters.clone(),
            );
            debug!("self_metrics: {:#?}", data);

            // Send all data
            info!("{}: Send data", Utc::now().format("%Y-%m-%dT%H:%M:%S"));
            for msg in &data {
                rclient.send_metric(msg);
            }

            thread::sleep(Duration::new(dispatch_duration, 0));
        }
    }

    /// Returns options understood by the exporter.
    fn get_options() -> HashMap<String, ExporterOption> {
        let mut options = HashMap::new();

        options.insert(
            String::from("address"),
            ExporterOption {
                default_value: Some(String::from(DEFAULT_IP_ADDRESS)),
                help: String::from("Riemann ipv6 or ipv4 address"),
                long: String::from("address"),
                short: String::from("a"),
                required: false,
                takes_value: true,
            },
        );
        options.insert(
            String::from("port"),
            ExporterOption {
                default_value: Some(String::from(DEFAULT_PORT)),
                help: String::from("Riemann TCP port number"),
                long: String::from("port"),
                short: String::from("p"),
                required: false,
                takes_value: true,
            },
        );
        options.insert(
            String::from("dispatch_duration"),
            ExporterOption {
                default_value: Some(String::from("5")),
                help: String::from("Duration between metrics dispatch"),
                long: String::from("dispatch"),
                short: String::from("d"),
                required: false,
                takes_value: true,
            },
        );
        options.insert(
            String::from("qemu"),
            ExporterOption {
                default_value: None,
                help: String::from("Instruct that scaphandre is running on an hypervisor"),
                long: String::from("qemu"),
                short: String::from("q"),
                required: false,
                takes_value: false,
            },
        );

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
