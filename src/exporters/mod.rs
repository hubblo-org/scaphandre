pub mod json;
pub mod prometheus;
pub mod qemu;
pub mod riemann;
pub mod stdout;
pub mod utils;
pub mod warpten;
use crate::sensors::{Record, RecordGenerator, Topology};
use clap::ArgMatches;
use std::collections::HashMap;
use std::fmt;
use utils::get_scaphandre_version;

#[derive(Debug)]
pub struct Metric {
    // timestamp: TBD is the timestamp must be in the struct here ?
    // Or computed just before sending the metric
    name: String, // Will be used as service for Riemann
    metric_type: String,
    ttl: f32,
    hostname: String,
    state: String,
    tags: Vec<String>,
    attributes: HashMap<String, String>,
    description: String,
    metric_value: MetricValueType,
}

enum MetricValueType {
    // IntSigned(i64),
    // Float(f32),
    Text(String),
    FloatDouble(f64),
    IntUnsigned(u64),
}

impl fmt::Debug for MetricValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            // MetricValueType::IntSigned(value) => write!(f, "{}", value),
            // MetricValueType::Float(value) => write!(f, "{}", value),
            MetricValueType::Text(text) => write!(f, "{}", text),
            MetricValueType::FloatDouble(value) => write!(f, "{}", value),
            MetricValueType::IntUnsigned(value) => write!(f, "{}", value),
        }
    }
}

/// An Exporter is what tells scaphandre when to collect metrics and how to export
/// or expose them.
/// Its basic role is to instanciate a Sensor, get the data the sensor has to offer
/// and expose the data in the desired way. An exporter could either push the metrics
/// over the network to a remote destination, store those metrics on the filesystem
/// or expose them to be collected by another software. It decides at what pace
/// the metrics are generated/refreshed by calling the refresh* methods available
/// with the structs provided by the sensor.
pub trait Exporter {
    fn run(&mut self, parameters: ArgMatches);
    fn get_options() -> HashMap<String, ExporterOption>;
}

pub struct ExporterOption {
    /// States whether the option is mandatory or not
    pub required: bool,
    /// Does the option need a value to be specified ?
    pub takes_value: bool,
    /// The default value, if needed
    pub default_value: Option<String>,
    /// One letter to identify the option (useful for the CLI)
    pub short: String,
    /// A word to identify the option
    pub long: String,
    /// A brief description to explain what the option does
    pub help: String,
}

struct MetricGenerator;

impl MetricGenerator {
    // Due to the fact that we collect all metric then send at the end,
    // client does not need to be passed. It is definitively a simpler approach
    /// Retrieve all scaphandre self metrics
    fn get_self_metrics(&self, topology: &Topology, data: &mut Vec<Metric>, hostname: &str) {
        data.push(Metric {
            name: String::from("scaph_self_version"),
            metric_type: String::from("gauge"),
            ttl: 60.0,
            hostname: String::from(hostname),
            state: String::from("ok"),
            tags: vec!["scaphandre".to_string()],
            attributes: HashMap::new(),
            description: String::from("Version number of scaphandre represented as a float."),
            metric_value: MetricValueType::Text(get_scaphandre_version()),
        });

        if let Some(metric_value) = topology
            .get_process_cpu_consumption_percentage(procfs::process::Process::myself().unwrap().pid)
        {
            data.push(Metric {
                name: String::from("scaph_self_cpu_usage_percent"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                hostname: String::from(hostname),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                description: String::from("CPU % consumed by this scaphandre exporter."),
                metric_value: MetricValueType::FloatDouble(metric_value),
            });
        }

        if let Ok(metric_value) = procfs::process::Process::myself().unwrap().statm() {
            let value = metric_value.size * procfs::page_size().unwrap() as u64;
            data.push(Metric {
                name: String::from("scaph_self_mem_total_program_size"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                hostname: String::from(hostname),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                // TODO: Do not use pages but human readable value (KB or MB)
                description: String::from("Total program size, measured in pages."),
                metric_value: MetricValueType::IntUnsigned(value),
            });

            let value = metric_value.resident * procfs::page_size().unwrap() as u64;
            data.push(Metric {
                name: String::from("scaph_self_mem_resident_set_size"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                hostname: String::from(hostname),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                // TODO: Do not use pages but human readable value (KB or MB)
                description: String::from("Resident set size, measured in pages."),
                metric_value: MetricValueType::IntUnsigned(value),
            });

            let value = metric_value.shared * procfs::page_size().unwrap() as u64;
            data.push(Metric {
                name: String::from("scaph_self_mem_shared_resident_size"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                hostname: String::from(hostname),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                // TODO: Do not use pages but human readable value (KB or MB)
                description: String::from(
                    "Number of resident shared pages (i.e., backed by a file).",
                ),
                metric_value: MetricValueType::IntUnsigned(value),
            });
        }

        let topo_stat_buffer_len = topology.stat_buffer.len();
        let topo_record_buffer_len = topology.record_buffer.len();
        let topo_procs_len = topology.proc_tracker.procs.len();

        data.push(Metric {
            name: String::from("scaph_self_topo_stats_nb"),
            metric_type: String::from("gauge"),
            ttl: 60.0,
            hostname: String::from(hostname),
            state: String::from("ok"),
            tags: vec!["scaphandre".to_string()],
            attributes: HashMap::new(),
            description: String::from("Number of CPUStat traces stored for the host."),
            metric_value: MetricValueType::IntUnsigned(topo_stat_buffer_len as u64),
        });

        data.push(Metric {
            name: String::from("scaph_self_topo_records_nb"),
            metric_type: String::from("gauge"),
            ttl: 60.0,
            hostname: String::from(hostname),
            state: String::from("ok"),
            tags: vec!["scaphandre".to_string()],
            attributes: HashMap::new(),
            description: String::from("Number of energy consumption Records stored for the host."),
            metric_value: MetricValueType::IntUnsigned(topo_record_buffer_len as u64),
        });

        data.push(Metric {
            name: String::from("scaph_self_topo_procs_nb"),
            metric_type: String::from("gauge"),
            ttl: 60.0,
            hostname: String::from(hostname),
            state: String::from("ok"),
            tags: vec!["scaphandre".to_string()],
            attributes: HashMap::new(),
            description: String::from("Number of processes monitored for the host."),
            metric_value: MetricValueType::IntUnsigned(topo_procs_len as u64),
        });
    }

    fn get_host_metrics(
        &self,
        topology: &Topology,
        data: &mut Vec<Metric>,
        hostname: &str,
        records: &[Record],
    ) {
        for socket in &topology.sockets {
            let mut attributes = HashMap::new();
            attributes.insert("socket_id".to_string(), socket.id.to_string());

            data.push(Metric {
                name: String::from("scaph_self_socket_stats_nb"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                hostname: String::from(hostname),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: attributes.clone(),
                description: String::from("Number of CPUStat traces stored for each socket"),
                metric_value: MetricValueType::IntUnsigned(socket.stat_buffer.len() as u64),
            });

            data.push(Metric {
                name: String::from("scaph_self_socket_records_nb"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                hostname: String::from(hostname),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: attributes.clone(),
                description: String::from(
                    "Number of energy consumption Records stored for each socket",
                ),
                // TODO: Look to be the same metric as above, need to check. Removal ?
                metric_value: MetricValueType::IntUnsigned(socket.stat_buffer.len() as u64),
            });

            for domain in &socket.domains {
                let mut attributes = HashMap::new();
                attributes.insert("rapl_domain_name".to_string(), domain.name.to_string());

                data.push(Metric {
                    name: String::from("scaph_self_domain_records_nb"),
                    metric_type: String::from("gauge"),
                    ttl: 60.0,
                    hostname: String::from(hostname),
                    state: String::from("ok"),
                    tags: vec!["scaphandre".to_string()],
                    attributes,
                    description: String::from(
                        "Number of energy consumption Records stored for a Domain",
                    ),
                    metric_value: MetricValueType::IntUnsigned(domain.record_buffer.len() as u64),
                });
            }
        }

        // metrics
        if !records.is_empty() {
            let record = records.last().unwrap();
            let host_energy_microjoules = record.value.clone();
            let host_energy_timestamp_seconds = record.timestamp.as_secs().to_string();

            data.push(Metric {
                    name: String::from("scaph_host_energy_microjoules"),
                    metric_type: String::from("gauge"),
                    ttl: 60.0,
                    hostname: String::from(hostname),
                    state: String::from("ok"),
                    tags: vec!["scaphandre".to_string()],
                    attributes: HashMap::new(),
                    description: String::from(
                        "Energy measurement for the whole host, as extracted from the sensor, in microjoules.",
                    ),
                    metric_value: MetricValueType::Text(host_energy_microjoules),
                });

            data.push(Metric {
                name: String::from("scaph_host_energy_timestamp_seconds"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                hostname: String::from(hostname),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                description: String::from(
                    "Timestamp in seconds when host_energy_microjoules has been computed.",
                ),
                metric_value: MetricValueType::Text(host_energy_timestamp_seconds),
            });

            if let Some(power) = topology.get_records_diff_power_microwatts() {
                data.push(Metric {
                    name: String::from("scaph_host_power_microwatts"),
                    metric_type: String::from("gauge"),
                    ttl: 60.0,
                    hostname: String::from(hostname),
                    state: String::from("ok"),
                    tags: vec!["scaphandre".to_string()],
                    attributes: HashMap::new(),
                    description: String::from("Power measurement on the whole host, in microwatts"),
                    metric_value: MetricValueType::Text(power.value),
                });
            }
        }
    }

    fn get_socket_metrics(&self, topology: &Topology, data: &mut Vec<Metric>, hostname: &str) {
        let sockets = topology.get_sockets_passive();
        for socket in sockets {
            let records = socket.get_records_passive();
            if !records.is_empty() {
                let socket_energy_microjoules = &records.last().unwrap().value;

                let mut attributes = HashMap::new();
                attributes.insert("socket_id".to_string(), socket.id.to_string());

                data.push(Metric {
                    name: String::from("scaph_socket_energy_microjoules"),
                    metric_type: String::from("gauge"),
                    ttl: 60.0,
                    hostname: String::from(hostname),
                    state: String::from("ok"),
                    tags: vec!["scaphandre".to_string()],
                    attributes: attributes.clone(),
                    description: String::from("Socket related energy measurement in microjoules."),
                    metric_value: MetricValueType::Text(socket_energy_microjoules.clone()),
                });

                if let Some(power) = topology.get_records_diff_power_microwatts() {
                    let socket_power_microwatts = &power.value;

                    data.push(Metric {
                        name: String::from("scaph_socket_power_microwatts"),
                        metric_type: String::from("gauge"),
                        ttl: 60.0,
                        hostname: String::from(hostname),
                        state: String::from("ok"),
                        tags: vec!["scaphandre".to_string()],
                        attributes: attributes.clone(),
                        description: String::from(
                            "Power measurement relative to a CPU socket, in microwatts",
                        ),
                        metric_value: MetricValueType::Text(socket_power_microwatts.clone()),
                    });
                }
            }
        }
    }

    fn get_system_metrics(&self, topology: &Topology, data: &mut Vec<Metric>, hostname: &str) {
        if let Some(metric_value) = topology.read_nb_process_total_count() {
            data.push(Metric {
                name: String::from("scaph_forks_since_boot_total"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                hostname: String::from(hostname),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                description: String::from("Number of forks that have occured since boot (number of processes to have existed so far)."),
                metric_value: MetricValueType::IntUnsigned(metric_value),
            });
        }

        if let Some(metric_value) = topology.read_nb_process_running_current() {
            data.push(Metric {
                name: String::from("scaph_processes_running_current"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                hostname: String::from(hostname),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                description: String::from("Number of processes currently running."),
                metric_value: MetricValueType::IntUnsigned(metric_value as u64),
            });
        }

        if let Some(metric_value) = topology.read_nb_process_blocked_current() {
            data.push(Metric {
                name: String::from("scaph_processes_blocked_current"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                hostname: String::from(hostname),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                description: String::from("Number of processes currently blocked waiting for I/O."),
                metric_value: MetricValueType::IntUnsigned(metric_value as u64),
            });
        }

        if let Some(metric_value) = topology.read_nb_context_switches_total_count() {
            data.push(Metric {
                name: String::from("scaph_context_switches_total"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                hostname: String::from(hostname),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                description: String::from("Number of context switches since boot."),
                metric_value: MetricValueType::IntUnsigned(metric_value as u64),
            });
        }
    }

    fn get_process_metrics(
        &self,
        topology: &Topology,
        data: &mut Vec<Metric>,
        hostname: &str,
        parameters: ArgMatches,
    ) {
        let processes_tracker = &topology.proc_tracker;

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
                    hostname: String::from(hostname),
                    state: String::from("ok"),
                    tags: vec!["scaphandre".to_string()],
                    attributes,
                    description: String::from("Power consumption due to the process, measured on at the topology level, in microwatts"),
                    metric_value: MetricValueType::Text(power.to_string()),
                });
            }
        }
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
