use crate::exporters::*;
use crate::sensors::Sensor;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

/// An Exporter that writes power consumption data of the host
/// and its processes in the JSON format, either in a file or
/// to the standard output.
pub struct JsonExporter {
    metric_generator: MetricGenerator,
    time_step: Duration,
    time_limit: Option<Duration>,
    max_top_consumers: u16,
    out_writer: BufWriter<Box<dyn Write>>,
    process_regex: Option<Regex>,
    container_regex: Option<Regex>,
    monitor_resources: bool,
    watch_containers: bool,
}

// Note: clap::Args automatically generate Args for the fields of this struct,
// using the field's name as the argument's name, and the doc comment
// above the field as the argument's description.

/// Holds the arguments for a JsonExporter.
///
/// When using Scaphandre as a command-line application, such a struct will be
/// automatically populated by the clap library. If you're using Scaphandre as
/// a library, you should populate the arguments yourself.
#[derive(clap::Args, Debug)]
pub struct ExporterArgs {
    /// Maximum time spent measuring, in seconds.
    /// If unspecified, runs forever.
    #[arg(short, long)]
    pub timeout: Option<i64>,

    /// Interval between two measurements, in seconds
    #[arg(short, long, value_name = "SECONDS", default_value_t = 2)]
    pub step: u64,

    /// Additional step duration in _nano_ seconds.
    /// This is added to `step` to get the final duration.
    #[arg(long, value_name = "NANOSECS", default_value_t = 0)]
    pub step_nano: u32,

    /// Maximum number of processes to watch
    #[arg(long, default_value_t = 10)]
    pub max_top_consumers: u16,

    /// Destination file for the report (if absent, print the report to stdout)
    #[arg(short, long)]
    pub file: Option<String>,

    /// Monitor and apply labels for processes running as containers
    #[arg(long)]
    pub containers: bool,

    /// Filter processes based on regular expressions (example: 'scaph\\w\\w.e')
    #[arg(long)]
    pub process_regex: Option<Regex>,

    /// Filter containers based on regular expressions
    #[arg(long)]
    pub container_regex: Option<Regex>,

    /// Monitor and incude CPU, RAM and Disk usage per process
    #[arg(long)]
    pub resources: bool,
    // TODO uncomment this option once we display something interesting about it
    // /// Apply labels to metrics of processes looking like a Qemu/KVM virtual machine
    // #[arg(short, long)]
    // pub qemu: bool
}

// Below are the structures that will store the reports.

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

impl Exporter for JsonExporter {
    /// Runs [iterate()] every `step` until `timeout`
    fn run(&mut self) {
        let step = self.time_step;
        info!("Measurement step is: {step:?}");

        if let Some(timeout) = self.time_limit {
            let t0 = Instant::now();
            while t0.elapsed() <= timeout {
                self.iterate();
                thread::sleep(self.time_step);
            }
        } else {
            loop {
                self.iterate();
                thread::sleep(self.time_step);
            }
        }
    }

    fn kind(&self) -> &str {
        "json"
    }
}

impl JsonExporter {
    /// Instantiates and returns a new JsonExporter.
    pub fn new(sensor: &dyn Sensor, args: ExporterArgs) -> JsonExporter {
        // Prepare the retrieval of the measurements
        let topo = sensor
            .get_topology()
            .expect("sensor topology should be available");
        let metric_generator =
            MetricGenerator::new(topo, utils::get_hostname(), false, args.containers);

        // Extract the parameters we need to run the exporter
        let time_step = Duration::new(args.step, args.step_nano);
        let time_limit;
        if let Some(t) = args.timeout {
            time_limit = Some(Duration::from_secs(t.unsigned_abs()))
        } else {
            time_limit = None
        };
        let max_top_consumers = args.max_top_consumers;
        let process_regex = args.process_regex;
        let container_regex = args.container_regex;
        let monitor_resources = args.resources;

        // Prepare the output (either stdout or a file)
        let output: Box<dyn Write> = match args.file {
            Some(f) => {
                let path = Path::new(&f);
                Box::new(File::create(path).unwrap_or_else(|_| panic!("failed to open file {f}")))
            }
            None => Box::new(std::io::stdout()),
        };
        let out_writer = BufWriter::new(output);
        JsonExporter {
            metric_generator,
            time_step,
            time_limit,
            max_top_consumers,
            out_writer,
            process_regex,
            container_regex,
            monitor_resources,
            watch_containers: args.containers,
        }
    }

    fn gen_disks_report(&self, metrics: &Vec<&Metric>) -> Vec<Disk> {
        let mut res: Vec<Disk> = vec![];
        for m in metrics {
            let metric_disk_name = m.attributes.get("disk_name").unwrap();
            if let Some(disk) = res.iter_mut().find(|x| metric_disk_name == &x.disk_name) {
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

    fn iterate(&mut self) {
        self.metric_generator.topology.refresh();
        self.retrieve_metrics();
    }

    fn retrieve_metrics(&mut self) {
        self.metric_generator.gen_all_metrics();

        let metrics = self.metric_generator.pop_metrics();
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
            // TODO in that case, no report is written, thus I think we should return here (?)
        };

        if let Some(host) = &mut host_report {
            host.components.disks = Some(disks);
        }

        let max_top = self.max_top_consumers;
        let consumers: Vec<(IProcess, f64)> = if let Some(regex_filter) = &self.process_regex {
            debug!("Processes filtered by '{}':", regex_filter.as_str());
            self.metric_generator
                .topology
                .proc_tracker
                .get_filtered_processes(regex_filter)
        } else if let Some(regex_filter) = &self.container_regex {
            debug!("Processes filtered by '{}':", regex_filter.as_str());
            #[cfg(feature = "containers")]
            {
                self.metric_generator
                    .get_processes_filtered_by_container_name(regex_filter)
            }

            #[cfg(not(feature = "containers"))]
            {
                self.metric_generator
                    .topology
                    .proc_tracker
                    .get_top_consumers(max_top)
            }
        } else {
            self.metric_generator
                .topology
                .proc_tracker
                .get_top_consumers(max_top)
        };

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
                        container: if self.watch_containers {
                            metric
                                .attributes
                                .get("container_id")
                                .map(|container_id| Container {
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
                                })
                        } else {
                            None
                        },
                    })
            })
            .collect::<Vec<_>>();

        if self.monitor_resources {
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

        let all_sockets = self
            .metric_generator
            .topology
            .get_sockets_passive()
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

                // Serialize the report to json
                serde_json::to_writer(&mut self.out_writer, &report)
                    .expect("report should be serializable to JSON");
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
