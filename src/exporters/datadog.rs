use crate::exporters::*;
use crate::sensors::{Sensor, Topology};
use datadog_client::client::{Client, Config};
use datadog_client::metrics::{Point, Serie, Type};
use std::collections::HashMap;
use std::thread;
use std::time::{Duration, Instant};

fn merge<A>(first: Vec<A>, second: Vec<A>) -> Vec<A> {
    second.into_iter().fold(first, |mut res, item| {
        res.push(item);
        res
    })
}

fn get_domain_name(index: usize) -> Option<&'static str> {
    match index {
        0 => Some("core"),
        1 => Some("uncore"),
        2 => Some("dram"),
        _ => None,
    }
}

/// An Exporter that displays power consumption data of the host
/// and its processes on the standard output of the terminal.
pub struct DatadogExporter {
    topology: Topology,
    hostname: String,
}

impl Exporter for DatadogExporter {
    /// Lanches runner()
    fn run(&mut self, parameters: ArgMatches) {
        self.runner(&parameters);
    }

    /// Returns options needed for that exporter, as a HashMap
    fn get_options() -> HashMap<String, ExporterOption> {
        let mut options = HashMap::new();
        options.insert(
            String::from("host"),
            ExporterOption {
                default_value: Some(String::from("https://api.datadoghq.eu")),
                long: String::from("host"),
                short: String::from("h"),
                required: true,
                takes_value: true,
                help: String::from("The domain of the datadog instance."),
            },
        );
        options.insert(
            String::from("api_key"),
            ExporterOption {
                default_value: None,
                long: String::from("api_key"),
                short: String::from("k"),
                required: true,
                takes_value: true,
                help: String::from("Api key to authenticate with datadog."),
            },
        );
        options
    }
}

impl DatadogExporter {
    /// Instantiates and returns a new DatadogExporter
    pub fn new(mut sensor: Box<dyn Sensor>) -> DatadogExporter {
        let some_topology = *sensor.get_topology();

        DatadogExporter {
            topology: some_topology.unwrap(),
            hostname: hostname::get()
                .expect("unable to get hostname")
                .to_str()
                .unwrap()
                .to_string(),
        }
    }

    fn build_client(parameters: &ArgMatches) -> Client {
        let config = Config::new(
            parameters.value_of("host").unwrap().to_string(),
            parameters.value_of("api_key").unwrap().to_string(),
        );
        Client::new(config)
    }

    fn runner(&mut self, parameters: &ArgMatches) {
        if let Some(timeout) = parameters.value_of("timeout") {
            let now = Instant::now();
            let timeout = timeout
                .parse::<u64>()
                .expect("Wrong timeout value, should be a number of seconds");

            // We have a default value of 2s so it is safe to unwrap the option
            // Panic if a non numerical value is passed
            let step_duration: u64 = parameters
                .value_of("step_duration")
                .unwrap()
                .parse::<u64>()
                .expect("Wrong step_duration value, should be a number of seconds");
            let step_duration_nano: u32 = parameters
                .value_of("step_duration_nano")
                .unwrap()
                .parse::<u32>()
                .expect("Wrong step_duration_nano value, should be a number of nano seconds");

            info!("Measurement step is: {}s", step_duration);

            while now.elapsed().as_secs() <= timeout {
                self.iterate(parameters);
                thread::sleep(Duration::new(step_duration, step_duration_nano));
            }
        } else {
            self.iterate(parameters);
        }
    }

    fn iterate(&mut self, parameters: &ArgMatches) {
        self.topology.refresh();
        let _series = self.collect_series();
        let _client = Self::build_client(parameters);
    }

    fn create_consumption_serie(&self) -> Serie {
        Serie::new("consumption", Type::Gauge)
            .set_host(self.hostname.as_str())
            .add_tag(format!("hostname:{}", self.hostname))
    }

    fn collect_process_series(&mut self) -> Vec<Serie> {
        let record = match self.topology.get_records_diff_power_microwatts() {
            Some(item) => item,
            None => return vec![],
        };
        let host_stat = match self.topology.get_stats_diff() {
            Some(item) => item,
            None => return vec![],
        };
        let host_power_ts = record.timestamp.as_secs();
        let host_power = record.value.parse::<u64>().unwrap_or(0) as f32;

        let ticks_per_second = procfs::ticks_per_second().unwrap() as f32;

        let consumers = self.topology.proc_tracker.get_top_consumers(10);
        consumers
            .iter()
            .map(|item| {
                let host_time = host_stat.total_time_jiffies();
                let consumption = (item.1 as f32 / (host_time * ticks_per_second)) * host_power;
                let exe = item
                    .0
                    .exe()
                    .ok()
                    .and_then(|v| v.to_str().map(|s| s.to_string()))
                    .unwrap_or_default();
                let point = Point::new(host_power_ts, consumption as f64);
                self.create_consumption_serie()
                    .add_point(point)
                    .add_tag(format!("process.exe:{}", exe))
                    .add_tag(format!("process.pid:{}", item.0.pid()))
            })
            .collect::<Vec<_>>()
    }

    fn collect_socket_series(&mut self) -> Vec<Serie> {
        self.topology
            .get_sockets_passive()
            .iter()
            .fold(Vec::new(), |mut res, socket| {
                let socket_record = match socket.get_records_diff_power_microwatts() {
                    Some(item) => item,
                    None => return res,
                };
                let socket_power = socket_record.value.parse::<u64>().unwrap_or(0);
                res.push(
                    self.create_consumption_serie()
                        .add_point(Point::new(
                            socket_record.timestamp.as_secs(),
                            socket_power as f64,
                        ))
                        .add_tag(format!("socket.id:{}", socket.id)),
                );
                socket
                    .get_domains_passive()
                    .iter()
                    .map(|d| d.get_records_diff_power_microwatts())
                    .enumerate()
                    .filter_map(|(index, record)| {
                        let name = match get_domain_name(index) {
                            Some(name) => name,
                            None => return None,
                        };
                        let record = match record {
                            Some(item) => item,
                            None => return None,
                        };
                        Some((
                            name,
                            Point::new(
                                record.timestamp.as_secs(),
                                record.value.parse::<u64>().unwrap_or(0) as f64,
                            ),
                        ))
                    })
                    .fold(res, |mut res, (name, point)| {
                        res.push(
                            self.create_consumption_serie()
                                .add_point(point)
                                .add_tag(format!("socket.id:{}", socket.id))
                                .add_tag(format!("socket.domain:{}", name)),
                        );
                        res
                    })
            })
    }

    fn collect_series(&mut self) -> Vec<Serie> {
        let processes = self.collect_process_series();
        let sockets = self.collect_socket_series();
        merge(processes, sockets)
    }
}

#[cfg(test)]
mod tests {
    //#[test]
    //fn get_cons_socket0() {}
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
