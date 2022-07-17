use crate::exporters::*;
use crate::sensors::Sensor;
use clap::Arg;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

/// An Exporter that displays power consumption data of the host
/// and its processes on the standard output of the terminal.
pub struct JSONExporter {
    sensor: Box<dyn Sensor>,
    reports: Vec<Report>,
}

impl Exporter for JSONExporter {
    /// Lanches runner()
    fn run(&mut self, parameters: ArgMatches) {
        self.runner(parameters);
    }

    /// Returns options needed for that exporter, as a HashMap

    fn get_options() -> Vec<clap::Arg<'static, 'static>> {
        let mut options = Vec::new();
        let arg = Arg::with_name("timeout")
            .help("Maximum time spent measuring, in seconds.")
            .long("timeout")
            .short("t")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("step_duration")
            .default_value("2")
            .help("Set measurement step duration in second.")
            .long("step")
            .short("s")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("step_duration_nano")
            .default_value("0")
            .help("Set measurement step duration in nano second.")
            .long("step_nano")
            .short("n")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("file_path")
            .default_value("")
            .help("Destination file for the report.")
            .long("file")
            .short("f")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("max_top_consumers")
            .default_value("10")
            .help("Maximum number of processes to watch.")
            .long("max-top-consumers")
            .short("m")
            .required(false)
            .takes_value(true);
        options.push(arg);

        // the resulting labels of this option are not yet used by this exporter, activate this option once we display something interesting about it
        //let arg = Arg::with_name("qemu")
        //    .help("Apply labels to metrics of processes looking like a Qemu/KVM virtual machine")
        //    .long("qemu")
        //    .short("q")
        //    .required(false)
        //    .takes_value(false);
        //options.push(arg);

        options
    }
}

#[derive(Serialize, Deserialize)]
struct Domain {
    name: String,
    consumption: f32,
    timestamp: f64,
}
#[derive(Serialize, Deserialize)]
struct Socket {
    id: u16,
    consumption: f32,
    domains: Vec<Domain>,
    timestamp: f64,
}

#[derive(Serialize, Deserialize)]
struct Consumer {
    exe: PathBuf,
    pid: i32,
    consumption: f32,
    timestamp: f64,
}
#[derive(Serialize, Deserialize)]
struct Host {
    consumption: f32,
    timestamp: f64,
}
#[derive(Serialize, Deserialize)]
struct Report {
    host: Host,
    consumers: Vec<Consumer>,
    sockets: Vec<Socket>,
}

impl JSONExporter {
    /// Instantiates and returns a new JSONExporter
    pub fn new(sensor: Box<dyn Sensor>) -> JSONExporter {
        JSONExporter {
            sensor,
            reports: Vec::new(),
        }
    }

    /// Runs iteration() every 'step', until 'timeout'
    pub fn runner(&mut self, parameters: ArgMatches) {
        let topology = self.sensor.get_topology().unwrap();
        let mut metric_generator = MetricGenerator::new(
            topology,
            utils::get_hostname(),
            parameters.is_present("qemu"),
            parameters.is_present("containers"),
        );

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
        if let Some(timeout) = parameters.value_of("timeout") {
            let now = Instant::now();

            let timeout_secs: u64 = timeout.parse().unwrap();
            while now.elapsed().as_secs() <= timeout_secs {
                self.iterate(&parameters, &mut metric_generator);
                thread::sleep(Duration::new(step_duration, step_duration_nano));
            }
        } else {
            loop {
                self.iterate(&parameters, &mut metric_generator);
                thread::sleep(Duration::new(step_duration, step_duration_nano));
            }
        }
    }

    fn iterate(&mut self, parameters: &ArgMatches, metric_generator: &mut MetricGenerator) {
        metric_generator.topology.refresh();
        self.retrieve_metrics(parameters, metric_generator);
    }

    fn retrieve_metrics(
        &mut self,
        parameters: &ArgMatches,
        metric_generator: &mut MetricGenerator,
    ) {
        metric_generator.gen_all_metrics();

        let metrics = metric_generator.pop_metrics();
        let mut metrics_iter = metrics.iter();
        let mut host_report: Option<Host> = None;
        if let Some(host_metric) = metrics_iter.find(|x| x.name == "scaph_host_power_microwatts") {
            let host_power_string = format!("{}", host_metric.metric_value);
            let host_power_f32 = host_power_string.parse::<f32>().unwrap();
            if host_power_f32 > 0.0 {
                host_report = Some(Host {
                    consumption: host_power_f32,
                    timestamp: host_metric.timestamp.as_secs_f64(),
                });
            }
        } else {
            info!("didn't find host metric");
        };

        let consumers = metric_generator.topology.proc_tracker.get_top_consumers(
            parameters
                .value_of("max_top_consumers")
                .unwrap_or("10")
                .parse::<u16>()
                .unwrap(),
        );
        let top_consumers = consumers
            .iter()
            .filter_map(|(process, _value)| {
                metrics
                    .iter()
                    .find(|x| {
                        x.name == "scaph_process_power_consumption_microwatts"
                            && process.pid
                                == x.attributes.get("pid").unwrap().parse::<i32>().unwrap()
                    })
                    .map(|metric| Consumer {
                        exe: PathBuf::from(metric.attributes.get("exe").unwrap()),
                        pid: process.pid,
                        consumption: format!("{}", metric.metric_value).parse::<f32>().unwrap(),
                        timestamp: metric.timestamp.as_secs_f64(),
                    })
            })
            .collect::<Vec<_>>();

        let all_sockets = metric_generator
            .topology
            .get_sockets_passive()
            .iter()
            .filter_map(|socket| {
                if let Some(metric) = metrics_iter.find(|x| {
                    if x.name == "scaph_socket_power_microwatts" {
                        socket.id
                            == x.attributes
                                .get("socket_id")
                                .unwrap()
                                .parse::<u16>()
                                .unwrap()
                    } else {
                        info!("socket not found ! ");
                        false
                    }
                }) {
                    let socket_power = format!("{}", metric.metric_value).parse::<f32>().unwrap();

                    let domains = metrics
                        .iter()
                        .filter(|x| {
                            x.name == "scaph_domain_power_microwatts"
                                && x.attributes
                                    .get("socket_id")
                                    .unwrap()
                                    .parse::<u16>()
                                    .unwrap()
                                    == socket.id
                        })
                        .map(|d| Domain {
                            name: d.name.clone(),
                            consumption: format!("{}", d.metric_value).parse::<f32>().unwrap(),
                            timestamp: d.timestamp.as_secs_f64(),
                        })
                        .collect::<Vec<_>>();

                    Some(Socket {
                        id: socket.id,
                        consumption: (socket_power as f32),
                        domains,
                        timestamp: metric.timestamp.as_secs_f64(),
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        match host_report {
            Some(host) => {
                let report = Report {
                    host,
                    consumers: top_consumers,
                    sockets: all_sockets,
                };

                let file_path = parameters.value_of("file_path").unwrap();
                // Print json
                if file_path.is_empty() {
                    let json: String =
                        serde_json::to_string(&report).expect("Unable to parse report");
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
            None => {
                info!("No data yet, didn't write report.");
            }
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
