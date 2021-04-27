//! # Exporter
//!
//! `Exporter` is the root for all exporters. It defines the [Exporter] trait
//! needed to implement an exporter.
pub mod json;
pub mod prometheus;
pub mod qemu;
pub mod riemann;
pub mod stdout;
pub mod utils;
pub mod warpten;
use crate::sensors::{RecordGenerator, Topology};
use chrono::Utc;
use clap::ArgMatches;
use std::collections::HashMap;
use std::fmt;
use utils::get_scaphandre_version;

/// General metric definition.
#[derive(Debug)]
struct Metric {
    /// `name` is the metric name, it will be used as service field for Riemann.
    name: String, // Will be used as service for Riemann
    /// `metric_type` mostly used by Prometheus, define is it is a gauge, counter...
    metric_type: String,
    /// `ttl` time to live for this metric used by Riemann.
    ttl: f32,
    /// `hostname` host that provides the metric.
    hostname: String,
    /// `state` used by Riemann, define a state like Ok or Ko regarding this metric.
    state: String,
    /// `tags` used by Riemann, tags attached to the metric.
    tags: Vec<String>,
    /// `attributes` used by exporters to better qualify the metric. In Prometheus context
    /// this is used as a metric tag (socket_id) : `scaph_self_socket_stats_nb{socket_id="0"} 2`.
    attributes: HashMap<String, String>,
    /// `description` metric description and units used.
    description: String,
    /// `metric_value` the value of the metric. This is possible to pass different types using
    /// [MetricValueType] enum. It allows to do specific exporter processing based on types
    /// allowing flexibility.
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
    /// Entry point for all Exporters
    fn run(&mut self, parameters: ArgMatches);
    /// Get the options passed via the command line
    fn get_options() -> Vec<clap::Arg<'static, 'static>>;
}

/// MetricGenerator is an exporter helper structure to collect Scaphandre metrics.
/// The goal is to provide a standard Vec\<Metric\> that can be used by exporters
/// to avoid code duplication.
struct MetricGenerator<'a> {
    /// `data` will be used to store the metrics retrieved.
    data: Vec<Metric>,
    /// `topology` is the system physical layout retrieve via the sensors crate with
    /// associated metrics.
    topology: &'a Topology,
    /// `hostname` is the system name where the metrics belongs.
    hostname: &'a str,
}

/// This is not mandatory to use MetricGenerator methods. Exporter can use dedicated
/// code into the [Exporter] run() method to collect metrics. However it is advised
/// to use the following methods to avoid discrepancies between exporters.
impl<'a> MetricGenerator<'a> {
    /// Returns a MetricGenerator instance that will host metrics.
    fn new(topology: &'a Topology, hostname: &'a str) -> MetricGenerator<'a> {
        let data = Vec::new();
        MetricGenerator {
            data,
            topology,
            hostname,
        }
    }

    /// Generate all scaphandre (self) metrics.
    fn gen_self_metrics(&mut self) {
        self.data.push(Metric {
            name: String::from("scaph_self_version"),
            metric_type: String::from("gauge"),
            ttl: 60.0,
            hostname: String::from(self.hostname),
            state: String::from("ok"),
            tags: vec!["scaphandre".to_string()],
            attributes: HashMap::new(),
            description: String::from("Version number of scaphandre represented as a float."),
            metric_value: MetricValueType::Text(get_scaphandre_version()),
        });

        if let Some(metric_value) = self
            .topology
            .get_process_cpu_consumption_percentage(procfs::process::Process::myself().unwrap().pid)
        {
            self.data.push(Metric {
                name: String::from("scaph_self_cpu_usage_percent"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                hostname: String::from(self.hostname),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                description: String::from("CPU % consumed by this scaphandre exporter."),
                metric_value: MetricValueType::FloatDouble(metric_value),
            });
        }

        if let Ok(metric_value) = procfs::process::Process::myself().unwrap().statm() {
            let value = metric_value.size * procfs::page_size().unwrap() as u64;
            self.data.push(Metric {
                name: String::from("scaph_self_mem_total_program_size"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                hostname: String::from(self.hostname),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                description: String::from("Total program size, measured in bytes."),
                metric_value: MetricValueType::IntUnsigned(value),
            });

            let value = metric_value.resident * procfs::page_size().unwrap() as u64;
            self.data.push(Metric {
                name: String::from("scaph_self_mem_resident_set_size"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                hostname: String::from(self.hostname),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                description: String::from("Resident set size, measured in bytes."),
                metric_value: MetricValueType::IntUnsigned(value),
            });

            let value = metric_value.shared * procfs::page_size().unwrap() as u64;
            self.data.push(Metric {
                name: String::from("scaph_self_mem_shared_resident_size"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                hostname: String::from(self.hostname),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                description: String::from(
                    "Number of resident shared bytes (i.e., backed by a file).",
                ),
                metric_value: MetricValueType::IntUnsigned(value),
            });
        }

        let topo_stat_buffer_len = self.topology.stat_buffer.len();
        let topo_record_buffer_len = self.topology.record_buffer.len();
        let topo_procs_len = self.topology.proc_tracker.procs.len();

        self.data.push(Metric {
            name: String::from("scaph_self_topo_stats_nb"),
            metric_type: String::from("gauge"),
            ttl: 60.0,
            hostname: String::from(self.hostname),
            state: String::from("ok"),
            tags: vec!["scaphandre".to_string()],
            attributes: HashMap::new(),
            description: String::from("Number of CPUStat traces stored for the host."),
            metric_value: MetricValueType::IntUnsigned(topo_stat_buffer_len as u64),
        });

        self.data.push(Metric {
            name: String::from("scaph_self_topo_records_nb"),
            metric_type: String::from("gauge"),
            ttl: 60.0,
            hostname: String::from(self.hostname),
            state: String::from("ok"),
            tags: vec!["scaphandre".to_string()],
            attributes: HashMap::new(),
            description: String::from("Number of energy consumption Records stored for the host."),
            metric_value: MetricValueType::IntUnsigned(topo_record_buffer_len as u64),
        });

        self.data.push(Metric {
            name: String::from("scaph_self_topo_procs_nb"),
            metric_type: String::from("gauge"),
            ttl: 60.0,
            hostname: String::from(self.hostname),
            state: String::from("ok"),
            tags: vec!["scaphandre".to_string()],
            attributes: HashMap::new(),
            description: String::from("Number of processes monitored for the host."),
            metric_value: MetricValueType::IntUnsigned(topo_procs_len as u64),
        });

        for socket in &self.topology.sockets {
            let mut attributes = HashMap::new();
            attributes.insert("socket_id".to_string(), socket.id.to_string());

            self.data.push(Metric {
                name: String::from("scaph_self_socket_stats_nb"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                hostname: String::from(self.hostname),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: attributes.clone(),
                description: String::from("Number of CPUStat traces stored for each socket"),
                metric_value: MetricValueType::IntUnsigned(socket.stat_buffer.len() as u64),
            });

            self.data.push(Metric {
                name: String::from("scaph_self_socket_records_nb"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                hostname: String::from(self.hostname),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: attributes.clone(),
                description: String::from(
                    "Number of energy consumption Records stored for each socket",
                ),
                metric_value: MetricValueType::IntUnsigned(socket.record_buffer.len() as u64),
            });

            for domain in &socket.domains {
                attributes.insert("rapl_domain_name".to_string(), domain.name.to_string());

                self.data.push(Metric {
                    name: String::from("scaph_self_domain_records_nb"),
                    metric_type: String::from("gauge"),
                    ttl: 60.0,
                    hostname: String::from(self.hostname),
                    state: String::from("ok"),
                    tags: vec!["scaphandre".to_string()],
                    attributes: attributes.clone(),
                    description: String::from(
                        "Number of energy consumption Records stored for a Domain",
                    ),
                    metric_value: MetricValueType::IntUnsigned(domain.record_buffer.len() as u64),
                });
            }
        }
    }

    /// Generate host metrics.
    fn gen_host_metrics(&mut self) {
        let records = self.topology.get_records_passive();

        // metrics
        if !records.is_empty() {
            let record = records.last().unwrap();
            let host_energy_microjoules = record.value.clone();
            let host_energy_timestamp_seconds = record.timestamp.as_secs().to_string();

            self.data.push(Metric {
                    name: String::from("scaph_host_energy_microjoules"),
                    metric_type: String::from("counter"),
                    ttl: 60.0,
                    hostname: String::from(self.hostname),
                    state: String::from("ok"),
                    tags: vec!["scaphandre".to_string()],
                    attributes: HashMap::new(),
                    description: String::from(
                        "Energy measurement for the whole host, as extracted from the sensor, in microjoules.",
                    ),
                    metric_value: MetricValueType::Text(host_energy_microjoules),
                });

            self.data.push(Metric {
                name: String::from("scaph_host_energy_timestamp_seconds"),
                metric_type: String::from("counter"),
                ttl: 60.0,
                hostname: String::from(self.hostname),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                description: String::from(
                    "Timestamp in seconds when host_energy_microjoules has been computed.",
                ),
                metric_value: MetricValueType::Text(host_energy_timestamp_seconds),
            });

            if let Some(power) = self.topology.get_records_diff_power_microwatts() {
                self.data.push(Metric {
                    name: String::from("scaph_host_power_microwatts"),
                    metric_type: String::from("gauge"),
                    ttl: 60.0,
                    hostname: String::from(self.hostname),
                    state: String::from("ok"),
                    tags: vec!["scaphandre".to_string()],
                    attributes: HashMap::new(),
                    description: String::from("Power measurement on the whole host, in microwatts"),
                    metric_value: MetricValueType::Text(power.value),
                });
            }
        }
    }

    /// Generate socket metrics.
    fn gen_socket_metrics(&mut self) {
        let sockets = self.topology.get_sockets_passive();
        for socket in sockets {
            let records = socket.get_records_passive();
            if !records.is_empty() {
                let socket_energy_microjoules = &records.last().unwrap().value;

                let mut attributes = HashMap::new();
                attributes.insert("socket_id".to_string(), socket.id.to_string());

                self.data.push(Metric {
                    name: String::from("scaph_socket_energy_microjoules"),
                    metric_type: String::from("counter"),
                    ttl: 60.0,
                    hostname: String::from(self.hostname),
                    state: String::from("ok"),
                    tags: vec!["scaphandre".to_string()],
                    attributes: attributes.clone(),
                    description: String::from("Socket related energy measurement in microjoules."),
                    metric_value: MetricValueType::Text(socket_energy_microjoules.clone()),
                });

                if let Some(power) = self.topology.get_records_diff_power_microwatts() {
                    let socket_power_microwatts = &power.value;

                    self.data.push(Metric {
                        name: String::from("scaph_socket_power_microwatts"),
                        metric_type: String::from("gauge"),
                        ttl: 60.0,
                        hostname: String::from(self.hostname),
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

    /// Generate system metrics.
    fn gen_system_metrics(&mut self) {
        if let Some(metric_value) = self.topology.read_nb_process_total_count() {
            self.data.push(Metric {
                name: String::from("scaph_forks_since_boot_total"),
                metric_type: String::from("counter"),
                ttl: 60.0,
                hostname: String::from(self.hostname),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                description: String::from("Number of forks that have occured since boot (number of processes to have existed so far)."),
                metric_value: MetricValueType::IntUnsigned(metric_value),
            });
        }

        if let Some(metric_value) = self.topology.read_nb_process_running_current() {
            self.data.push(Metric {
                name: String::from("scaph_processes_running_current"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                hostname: String::from(self.hostname),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                description: String::from("Number of processes currently running."),
                metric_value: MetricValueType::IntUnsigned(metric_value as u64),
            });
        }

        if let Some(metric_value) = self.topology.read_nb_process_blocked_current() {
            self.data.push(Metric {
                name: String::from("scaph_processes_blocked_current"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                hostname: String::from(self.hostname),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                description: String::from("Number of processes currently blocked waiting for I/O."),
                metric_value: MetricValueType::IntUnsigned(metric_value as u64),
            });
        }

        if let Some(metric_value) = self.topology.read_nb_context_switches_total_count() {
            self.data.push(Metric {
                name: String::from("scaph_context_switches_total"),
                metric_type: String::from("counter"),
                ttl: 60.0,
                hostname: String::from(self.hostname),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                description: String::from("Number of context switches since boot."),
                metric_value: MetricValueType::IntUnsigned(metric_value as u64),
            });
        }
    }

    /// Generate process metrics.
    fn gen_process_metrics(&mut self, qemu: bool) {
        let processes_tracker = &self.topology.proc_tracker;

        for pid in processes_tracker.get_alive_pids() {
            let exe = processes_tracker.get_process_name(pid);
            let cmdline = processes_tracker.get_process_cmdline(pid);

            let mut attributes = HashMap::new();
            attributes.insert("pid".to_string(), pid.to_string());

            attributes.insert("exe".to_string(), exe.clone());

            if let Some(cmdline_str) = cmdline {
                attributes.insert("cmdline".to_string(), cmdline_str.replace("\"", "\\\""));

                if qemu {
                    if let Some(vmname) = utils::filter_qemu_cmdline(&cmdline_str) {
                        attributes.insert("vmname".to_string(), vmname);
                    }
                }
            }

            let metric_name = String::from("scaph_process_power_consumption_microwatts");
            if let Some(power) = self.topology.get_process_power_consumption_microwatts(pid) {
                self.data.push(Metric {
                    name: metric_name,
                    metric_type: String::from("gauge"),
                    ttl: 60.0,
                    hostname: String::from(self.hostname),
                    state: String::from("ok"),
                    tags: vec!["scaphandre".to_string()],
                    attributes,
                    description: String::from("Power consumption due to the process, measured on at the topology level, in microwatts"),
                    metric_value: MetricValueType::Text(power.to_string()),
                });
            }
        }
    }

    /// Generate all metrics provided by Scaphandre agent.
    fn gen_all_metrics(&mut self, qemu: bool) {
        info!(
            "{}: Get self metrics",
            Utc::now().format("%Y-%m-%dT%H:%M:%S")
        );
        self.gen_self_metrics();
        info!(
            "{}: Get host metrics",
            Utc::now().format("%Y-%m-%dT%H:%M:%S")
        );
        self.gen_host_metrics();
        info!(
            "{}: Get socket metrics",
            Utc::now().format("%Y-%m-%dT%H:%M:%S")
        );
        self.gen_socket_metrics();
        info!(
            "{}: Get system metrics",
            Utc::now().format("%Y-%m-%dT%H:%M:%S")
        );
        self.gen_system_metrics();
        info!(
            "{}: Get process metrics",
            Utc::now().format("%Y-%m-%dT%H:%M:%S")
        );
        self.gen_process_metrics(qemu);
        debug!("self_metrics: {:#?}", self.data);
    }

    /// Retrieve the current metrics stored into [MetricGenerator].
    ///
    /// [MetricGenerator] is loaded using the gen_*_metrics() methods
    /// Most of the time gen_all_metrics() is used to extract the full
    /// set of data.
    fn get_metrics(&self) -> &Vec<Metric> {
        &self.data
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
