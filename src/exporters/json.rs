use crate::exporters::*;
use crate::sensors::Sensor;
use clap::{value_parser, Arg};
use colored::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    fs::File,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

/// An Exporter that displays power consumption data of the host
/// and its processes on the standard output of the terminal.
pub struct JSONExporter {
    sensor: Box<dyn Sensor>,
    reports: Vec<Report>,
    regex: Option<Regex>,
}

impl Exporter for JSONExporter {
    /// Lanches runner()
    fn run(&mut self, parameters: ArgMatches) {
        self.runner(parameters);
    }

    /// Returns options needed for that exporter, as a HashMap

    fn get_options() -> Vec<clap::Arg> {
        let mut options = Vec::new();
        let arg = Arg::new("timeout")
            .help("Maximum time spent measuring, in seconds.")
            .long("timeout")
            .short('t')
            .required(false)
            .value_parser(value_parser!(u64))
            .action(clap::ArgAction::Set);
        options.push(arg);

        let arg = Arg::new("step_duration")
            .default_value("2")
            .help("Set measurement step duration in second.")
            .long("step")
            .short('s')
            .required(false)
            .value_parser(value_parser!(u64))
            .action(clap::ArgAction::Set);
        options.push(arg);

        let arg = Arg::new("step_duration_nano")
            .default_value("0")
            .help("Set measurement step duration in nano second.")
            .long("step_nano")
            .short('n')
            .required(false)
            .value_parser(value_parser!(u32))
            .action(clap::ArgAction::Set);
        options.push(arg);

        let arg = Arg::new("file_path")
            .default_value("")
            .help("Destination file for the report.")
            .long("file")
            .short('f')
            .required(false)
            .action(clap::ArgAction::Set);
        options.push(arg);

        let arg = Arg::new("max_top_consumers")
            .default_value("10")
            .help("Maximum number of processes to watch.")
            .long("max-top-consumers")
            .short('m')
            .required(false)
            .value_parser(value_parser!(u16))
            .action(clap::ArgAction::Set);
        options.push(arg);

        #[cfg(feature = "containers")]
        {
            let arg = Arg::with_name("containers")
                .help("Monitor and apply labels for processes running as containers")
                .short("c")
                .long("containers")
                .required(false)
                .takes_value(false);
            options.push(arg);
        }

        let arg = Arg::with_name("regex_filter")
            .help("Filter processes based on regular expressions (e.g: 'scaph\\w\\wd.e').")
            .long("regex")
            .short("r")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("resources")
            .help("Monitor and include CPU/RAM/Disk usage per process.")
            .long("resources")
            .required(false);
        options.push(arg);

        #[cfg(feature = "containers")]
        {
            let arg = Arg::with_name("container_regex")
                .help("Filter process by container name based on regular expressions (e.g: 'scaph\\w\\wd.e'). Works only with --containers enabled.")
                .long("container-regex")
                .required(false)
                .takes_value(true);
            options.push(arg);
        }

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
    cmdline: String,
    pid: i32,
    resources_usage: Option<ResourcesUsage>,
    consumption: f32,
    timestamp: f64,
    container: Option<Container>,
}

#[derive(Serialize, Deserialize)]
struct ResourcesUsage {
    cpu_usage: String,
    cpu_usage_unit: String,
    memory_usage: String,
    memory_usage_unit: String,
    memory_virtual_usage: String,
    memory_virtual_usage_unit: String,
    disk_usage_write: String,
    disk_usage_write_unit: String,
    disk_usage_read: String,
    disk_usage_read_unit: String,
}

#[derive(Serialize, Deserialize)]
struct Container {
    name: String,
    id: String,
    runtime: String,
    scheduler: String,
}
#[derive(Serialize, Deserialize)]
struct Disk {
    disk_type: String,
    disk_mount_point: String,
    disk_is_removable: bool,
    disk_file_system: String,
    disk_total_bytes: String,
    disk_available_bytes: String,
    disk_name: String,
}
#[derive(Serialize, Deserialize)]
struct Components {
    disks: Option<Vec<Disk>>,
}
#[derive(Serialize, Deserialize)]
struct Host {
    consumption: f32,
    timestamp: f64,
    components: Components,
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
            regex: None,
        }
    }

    /// Runs iteration() every 'step', until 'timeout'
    pub fn runner(&mut self, parameters: ArgMatches) {
        let topology = self.sensor.get_topology().unwrap();
        let mut metric_generator = MetricGenerator::new(
            topology,
            utils::get_hostname(),
            parameters.get_flag("qemu"),
            parameters.get_flag("containers"),
        );

        // We have a default value of 2s so it is safe to unwrap the option
        // Panic if a non numerical value is passed
        let step_duration: u64 = *parameters
            .get_one("step_duration")
            .expect("Wrong step_duration value, should be a number of seconds");
        let step_duration_nano: u32 = *parameters
            .get_one("step_duration_nano")
            .expect("Wrong step_duration_nano value, should be a number of nano seconds");

        self.regex = if !parameters.is_present("regex_filter")
            || parameters.value_of("regex_filter").unwrap().is_empty()
        {
            None
        } else {
            Some(
                Regex::new(parameters.value_of("regex_filter").unwrap())
                    .expect("Wrong regex_filter, regexp is invalid"),
            )
        };

        if parameters.occurrences_of("regex_filter") == 1
            && parameters.occurrences_of("max_top_consumers") == 1
        {
            let warning =
                String::from("Warning: (--max-top-consumers) and (-r / --regex) used at the same time. (--max-top-consumers) disabled");
            eprintln!("{}", warning.bright_yellow());
        }

        #[cfg(feature = "containers")]
        if !parameters.is_present("containers") && parameters.is_present("container_regex") {
            let warning =
                String::from("Warning: --container-regex is used but --containers is not enabled. Regex search won't work.");
            eprintln!("{}", warning.bright_yellow());
        }

        info!("Measurement step is: {}s", step_duration);
        if let Some(timeout) = parameters.get_one::<u64>("timeout") {
            let now = Instant::now();

            let timeout_secs: u64 = *timeout;
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

    fn gen_disks_report(&self, metrics: &Vec<&Metric>) -> Vec<Disk> {
        let mut res: Vec<Disk> = vec![];
        for m in metrics {
            let metric_disk_name = m.attributes.get("disk_name").unwrap();
            if let Some(mut disk) = res.iter_mut().find(|x| metric_disk_name == &x.disk_name) {
                info!("editing disk");
                disk.disk_name = metric_disk_name.clone();
                if m.name == "scaph_host_disk_available_bytes" {
                    disk.disk_available_bytes = m.metric_value.to_string();
                } else if m.name == "scaph_host_disk_total_bytes" {
                    disk.disk_total_bytes = m.metric_value.to_string();
                }
            } else {
                info!("adding disk");
                res.push(Disk {
                    disk_name: metric_disk_name.clone(),
                    disk_available_bytes: {
                        if m.name == "scaph_host_disk_available_bytes" {
                            m.metric_value.to_string()
                        } else {
                            String::from("")
                        }
                    },
                    disk_file_system: {
                        if let Some(metric_disk_file_system) = m.attributes.get("disk_file_system")
                        {
                            metric_disk_file_system.clone()
                        } else {
                            String::from("")
                        }
                    },
                    disk_is_removable: {
                        if let Some(metric_disk_is_removable) =
                            m.attributes.get("disk_is_removable")
                        {
                            metric_disk_is_removable.parse::<bool>().unwrap()
                        } else {
                            false
                        }
                    },
                    disk_mount_point: {
                        if let Some(metric_disk_mount_point) = m.attributes.get("disk_mount_point")
                        {
                            metric_disk_mount_point.clone()
                        } else {
                            String::from("")
                        }
                    },
                    disk_total_bytes: {
                        if m.name == "scaph_host_disk_total_bytes" {
                            m.metric_value.to_string()
                        } else {
                            String::from("")
                        }
                    },
                    disk_type: {
                        if let Some(metric_disk_type) = m.attributes.get("disk_type") {
                            metric_disk_type.clone()
                        } else {
                            String::from("")
                        }
                    },
                })
            }
        }
        res
    }

    fn retrieve_metrics(
        &mut self,
        parameters: &ArgMatches,
        metric_generator: &mut MetricGenerator,
    ) {
        metric_generator.gen_all_metrics();

        let metrics = metric_generator.pop_metrics();
        let mut metrics_iter = metrics.iter();
        let socket_metrics_res = metrics_iter.find(|x| x.name == "scaph_socket_power_microwatts");
        //TODO: fix for multiple sockets
        let mut host_report: Option<Host> = None;
        let disks = self.gen_disks_report(
            &metrics_iter
                .filter(|x| x.name.starts_with("scaph_host_disk_"))
                .collect(),
        );
        if let Some(host_metric) = &metrics
            .iter()
            .find(|x| x.name == "scaph_host_power_microwatts")
        {
            let host_power_string = format!("{}", host_metric.metric_value);
            let host_power_f32 = host_power_string.parse::<f32>().unwrap();
            if host_power_f32 > 0.0 {
                host_report = Some(Host {
                    consumption: host_power_f32,
                    timestamp: host_metric.timestamp.as_secs_f64(),
                    components: Components { disks: None },
                });
            }
        } else {
            info!("didn't find host metric");
        };

        if let Some(host) = &mut host_report {
            host.components.disks = Some(disks);
        }

        let consumers: Vec<(IProcess, f64)>;
        let max_top = parameters
            .value_of("max_top_consumers")
            .unwrap_or("10")
            .parse::<u16>()
            .unwrap();
        if let Some(regex_filter) = &self.regex {
            debug!("Processes filtered by '{}':", regex_filter.as_str());
            consumers = metric_generator
                .topology
                .proc_tracker
                .get_filtered_processes(regex_filter);
        } else if parameters.is_present("container_regex") {
            #[cfg(feature = "containers")]
            {
                consumers = metric_generator.get_processes_filtered_by_container_name(
                    &Regex::new(parameters.value_of("container_regex").unwrap())
                        .expect("Wrong container_regex expression. Regexp is invalid."),
                );
            }
            #[cfg(not(feature = "containers"))]
            {
                consumers = metric_generator
                    .topology
                    .proc_tracker
                    .get_top_consumers(max_top);
            }
        } else {
            consumers = metric_generator
                .topology
                .proc_tracker
                .get_top_consumers(max_top);
        }
        let mut top_consumers = consumers
            .iter()
            .filter_map(|(process, _value)| {
                metrics
                    .iter()
                    .find(|x| {
                        x.name == "scaph_process_power_consumption_microwatts"
                            && &process.pid.to_string() == x.attributes.get("pid").unwrap()
                    })
                    .map(|metric| Consumer {
                        exe: PathBuf::from(metric.attributes.get("exe").unwrap()),
                        cmdline: metric.attributes.get("cmdline").unwrap().clone(),
                        pid: process.pid.to_string().parse::<i32>().unwrap(),
                        consumption: format!("{}", metric.metric_value).parse::<f32>().unwrap(),
                        resources_usage: None,
                        timestamp: metric.timestamp.as_secs_f64(),
                        container: match parameters.get_flag("containers") {
                            true => metric.attributes.get("container_id").map(|container_id| {
                                Container {
                                    id: String::from(container_id),
                                    name: String::from(
                                        metric
                                            .attributes
                                            .get("container_names")
                                            .unwrap_or(&String::from("unknown")),
                                    ),
                                    runtime: String::from(
                                        metric
                                            .attributes
                                            .get("container_runtime")
                                            .unwrap_or(&String::from("unknown")),
                                    ),
                                    scheduler: String::from(
                                        metric
                                            .attributes
                                            .get("container_scheduler")
                                            .unwrap_or(&String::from("unknown")),
                                    ),
                                }
                            }),
                            false => None,
                        },
                    })
            })
            .collect::<Vec<_>>();

        if parameters.is_present("resources") {
            info!("ADDING RESOURCES");
            for c in top_consumers.iter_mut() {
                let mut res = ResourcesUsage {
                    cpu_usage: String::from("0"),
                    cpu_usage_unit: String::from("%"),
                    disk_usage_read: String::from("0"),
                    disk_usage_read_unit: String::from("Bytes"),
                    disk_usage_write: String::from("0"),
                    disk_usage_write_unit: String::from("Bytes"),
                    memory_usage: String::from("0"),
                    memory_usage_unit: String::from("Bytes"),
                    memory_virtual_usage: String::from("0"),
                    memory_virtual_usage_unit: String::from("Bytes"),
                };
                let mut metrics = metrics.iter().filter(|x| {
                    x.name.starts_with("scaph_process_")
                        && x.attributes.get("pid").unwrap() == &c.pid.to_string()
                });
                if let Some(cpu_usage_metric) =
                    metrics.find(|y| y.name == "scaph_process_cpu_usage_percentage")
                {
                    res.cpu_usage = cpu_usage_metric.metric_value.to_string();
                }
                if let Some(mem_usage_metric) =
                    metrics.find(|y| y.name == "scaph_process_memory_bytes")
                {
                    res.memory_usage = mem_usage_metric.metric_value.to_string();
                }
                if let Some(mem_virtual_usage_metric) =
                    metrics.find(|y| y.name == "scaph_process_memory_virtual_bytes")
                {
                    res.memory_virtual_usage = mem_virtual_usage_metric.metric_value.to_string();
                }
                if let Some(disk_write_metric) =
                    metrics.find(|y| y.name == "scaph_process_disk_write_bytes")
                {
                    res.disk_usage_write = disk_write_metric.metric_value.to_string();
                }
                if let Some(disk_read_metric) =
                    metrics.find(|y| y.name == "scaph_process_disk_read_bytes")
                {
                    res.disk_usage_read = disk_read_metric.metric_value.to_string();
                }
                c.resources_usage = Some(res);
            }
        }

        let all_sockets_vec = metric_generator.topology.get_sockets_passive();
        let all_sockets = all_sockets_vec
            .iter()
            .filter_map(|socket| {
                if let Some(metric) = socket_metrics_res.iter().find(|x| {
                    socket.id
                        == x.attributes
                            .get("socket_id")
                            .unwrap()
                            .parse::<u16>()
                            .unwrap()
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
                            name: d.attributes.get("domain_name").unwrap().clone(),
                            consumption: format!("{}", d.metric_value).parse::<f32>().unwrap(),
                            timestamp: d.timestamp.as_secs_f64(),
                        })
                        .collect::<Vec<_>>();

                    Some(Socket {
                        id: socket.id,
                        consumption: socket_power,
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

                let file_path = parameters.get_one::<String>("file_path").unwrap();
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
                    fs::write(file_path, json).expect("Unable to write file");
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
