use crate::exporters::*;
use crate::sensors::{Record, Sensor, Topology};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

/// An Exporter that displays power consumption data of the host
/// and its processes on the standard output of the terminal.
pub struct JSONExporter {
    topology: Topology,
    reports: Vec<Report>,
}

impl Exporter for JSONExporter {
    /// Lanches runner()
    fn run(&mut self, parameters: ArgMatches) {
        self.runner(parameters);
    }

    /// Returns options needed for that exporter, as a HashMap
    fn get_options() -> HashMap<String, ExporterOption> {
        let mut options = HashMap::new();
        options.insert(
            String::from("timeout"),
            ExporterOption {
                default_value: Some(String::from("10")),
                long: String::from("timeout"),
                short: String::from("t"),
                required: false,
                takes_value: true,
                help: String::from("Maximum time spent measuring, in seconds."),
            },
        );
        options.insert(
            String::from("step_duration"),
            ExporterOption {
                default_value: Some(String::from("2")),
                long: String::from("step"),
                short: String::from("s"),
                required: false,
                takes_value: true,
                help: String::from("Set measurement step duration in second."),
            },
        );
        options.insert(
            String::from("step_duration_nano"),
            ExporterOption {
                default_value: Some(String::from("0")),
                long: String::from("step_nano"),
                short: String::from("n"),
                required: false,
                takes_value: true,
                help: String::from("Set measurement step duration in nano second."),
            },
        );
        options.insert(
            String::from("file_path"),
            ExporterOption {
                default_value: Some(String::from("")),
                long: String::from("file"),
                short: String::from("f"),
                required: false,
                takes_value: true,
                help: String::from("Destination file for the report."),
            },
        );
        options
    }
}

#[derive(Serialize, Deserialize)]
struct Domain {
    name: String,
    consumption: f32,
}
#[derive(Serialize, Deserialize)]
struct Socket {
    id: u16,
    consumption: f32,
    domains: Vec<Domain>,
}

#[derive(Serialize, Deserialize)]
struct Consumer {
    exe: PathBuf,
    pid: i32,
    consumption: f32,
}
#[derive(Serialize, Deserialize)]
struct Report {
    host: f32,
    consumers: Vec<Consumer>,
    sockets: Vec<Socket>,
}

impl JSONExporter {
    /// Instantiates and returns a new JSONExporter
    pub fn new(mut sensor: Box<dyn Sensor>) -> JSONExporter {
        let some_topology = *sensor.get_topology();
        JSONExporter {
            topology: some_topology.unwrap(),
            reports: Vec::new(),
        }
    }

    /// Runs iteration() every 'step', until 'timeout'
    pub fn runner(&mut self, parameters: ArgMatches) {
        let timeout = parameters.value_of("timeout").unwrap();
        if timeout.is_empty() {
            self.iterate(&parameters);
        } else {
            let now = Instant::now();

            let timeout_secs: u64 = timeout.parse().unwrap();

            // We have a default value of 2s so it is safe to unwrap the option
            // Panic if a non numerical value is passed
            let step_duration: u64 = parameters
                .value_of("step_duration")
                .unwrap()
                .parse()
                .expect("Wrong step_duration value, should be a number of seconds");
            let step_duration_nano: u32 = parameters
                .value_of("step_duration_nano")
                .unwrap()
                .parse()
                .expect("Wrong step_duration_nano value, should be a number of nano seconds");

            info!("Measurement step is: {}s", step_duration);

            while now.elapsed().as_secs() <= timeout_secs {
                self.iterate(&parameters);
                thread::sleep(Duration::new(step_duration, step_duration_nano));
            }
        }
    }

    fn get_domains_power(&self, socket_id: u16) -> Vec<Option<Record>> {
        let socket_present = self
            .topology
            .get_sockets_passive()
            .iter()
            .find(move |x| x.id == socket_id);

        if let Some(socket) = socket_present {
            let mut domains_power: Vec<Option<Record>> = vec![];
            for d in socket.get_domains_passive() {
                domains_power.push(d.get_records_diff_power_microwatts());
            }
            domains_power
        } else {
            vec![None, None, None]
        }
    }

    fn iterate(&mut self, parameters: &ArgMatches) {
        self.topology.refresh();
        self.retrieve_metrics(&parameters);
    }

    fn retrieve_metrics(&mut self, parameters: &ArgMatches) {
        let host_power = match self.topology.get_records_diff_power_microwatts() {
            Some(record) => record.value.parse::<u64>().unwrap(),
            None => 0,
        };

        let consumers = self.topology.proc_tracker.get_top_consumers(10);
        let mut top_consumers = Vec::new();
        for c in consumers.iter() {
            if let Some(host_stat) = self.topology.get_stats_diff() {
                let host_time = host_stat.total_time_jiffies();
                let consumer = Consumer {
                    exe: c.0.exe().unwrap_or_default(),
                    pid: c.0.pid,
                    consumption: ((c.1 as f32
                        / (host_time * procfs::ticks_per_second().unwrap() as f32))
                        * host_power as f32),
                };
                top_consumers.push(consumer)
            }
        }

        let mut index = 0;
        let names = ["core", "uncore", "dram"];
        let mut all_sockets = Vec::new();
        let sockets = self.topology.get_sockets_passive();
        for s in sockets {
            let socket_power = match s.get_records_diff_power_microwatts() {
                Some(record) => record.value.parse::<u64>().unwrap(),
                None => 0,
            };

            let v = (socket_power, self.get_domains_power(s.id));
            let mut domains = Vec::new();

            for d in v.1.iter() {
                let domain_power = match d {
                    Some(record) => record.value.parse::<u64>().unwrap(),
                    None => 0,
                };

                domains.push(Domain {
                    name: names[index].to_string(),
                    consumption: domain_power as f32,
                });
                index += 1
            }

            all_sockets.push(Socket {
                id: s.id,
                consumption: (v.0 as f32),
                domains,
            });
        }

        let report = Report {
            host: host_power as f32,
            consumers: top_consumers,
            sockets: all_sockets,
        };

        let file_path = parameters.value_of("file_path").unwrap();
        // Print json
        if file_path.is_empty() {
            let json: String = serde_json::to_string(&report).expect("Unable to parse report");
            println!("{}", &json);
        } else {
            self.reports.push(report);
            // Serialize it to a JSON string.
            let json: String =
                serde_json::to_string(&self.reports).expect("Unable to parse report");
            let _ = File::create(file_path);
            fs::write(file_path, &json).expect("Unable to write file");
        }
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
