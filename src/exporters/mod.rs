pub mod json;
pub mod prometheus;
pub mod qemu;
pub mod riemann;
pub mod stdout;
pub mod utils;
pub mod warpten;
use crate::sensors::Topology;
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
    Int(usize),
    Float(f32),
    FloatDouble(f64),
    Text(String),
}

impl fmt::Debug for MetricValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            MetricValueType::Text(text) => write!(f, "{}", text),
            MetricValueType::Float(value) => write!(f, "{}", value),
            MetricValueType::FloatDouble(value) => write!(f, "{}", value),
            MetricValueType::Int(value) => write!(f, "{}", value),
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

    // Due to the fact that we collect all metric then send at the end,
    // client does not need to be passed. It is definitively a simpler approach
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
                description: String::from("CPU % consumed by this scaphandre prometheus exporter."),
                metric_value: MetricValueType::FloatDouble(metric_value),
            });
        }
    }
    //             if let Ok(metric_value) = procfs::process::Process::myself().unwrap().statm() {
    //                 let value = metric_value.size * procfs::page_size().unwrap() as u64;
    //                 rclient.send_metric(
    //                     60.0,
    //                     &hostname,
    //                     "scaph_self_mem_total_program_size",
    //                     "ok",
    //                     vec!["scaphandre".to_string()],
    //                     vec![],
    //                     "Total program size, measured in pages",
    //                     value,
    //                 );

    //                 let value = metric_value.resident * procfs::page_size().unwrap() as u64;
    //                 rclient.send_metric(
    //                     60.0,
    //                     &hostname,
    //                     "scaph_self_mem_resident_set_size",
    //                     "ok",
    //                     vec!["scaphandre".to_string()],
    //                     vec![],
    //                     "Resident set size, measured in pages",
    //                     value,
    //                 );

    //                 let value = metric_value.size * procfs::page_size().unwrap() as u64;
    //                 rclient.send_metric(
    //                     60.0,
    //                     &hostname,
    //                     "scaph_self_mem_shared_resident_size",
    //                     "ok",
    //                     vec!["scaphandre".to_string()],
    //                     vec![],
    //                     "Number of resident shared pages (i.e., backed by a file)",
    //                     value,
    //                 );
    //             }

    //             let topo_stat_buffer_len = topology.stat_buffer.len();
    //             let topo_record_buffer_len = topology.record_buffer.len();
    //             let topo_procs_len = topology.proc_tracker.procs.len();
    //             rclient.send_metric(
    //                 60.0,
    //                 &hostname,
    //                 "scaph_self_topo_stats_nb",
    //                 "ok",
    //                 vec!["scaphandre".to_string()],
    //                 vec![],
    //                 "Number of CPUStat traces stored for the host",
    //                 topo_stat_buffer_len,
    //             );

    //             rclient.send_metric(
    //                 60.0,
    //                 &hostname,
    //                 "scaph_self_topo_records_nb",
    //                 "ok",
    //                 vec!["scaphandre".to_string()],
    //                 vec![],
    //                 "Number of energy consumption Records stored for the host",
    //                 topo_record_buffer_len,
    //             );

    //             rclient.send_metric(
    //                 60.0,
    //                 &hostname,
    //                 "scaph_self_topo_procs_nb",
    //                 "ok",
    //                 vec!["scaphandre".to_string()],
    //                 vec![],
    //                 "Number of processes monitored for the host",
    //                 topo_procs_len,
    //             );

    //             rclient.send_metric(
    //                 60.0,
    //                 &hostname,
    //                 "scaph_self_topo_procs_nb",
    //                 "ok",
    //                 vec!["scaphandre".to_string()],
    //                 vec![],
    //                 "Number of processes monitored for the host",
    //                 topo_procs_len,
    //             );
    // }
    fn get_host_metrics(&self, topology: &Topology, data: &Vec<Metric>) {
        unimplemented!()
    }
    fn get_socket_metrics(&self, topology: &Topology, data: &Vec<Metric>) {
        unimplemented!()
    }
    fn get_system_metrics(&self, topology: &Topology, data: &Vec<Metric>) {
        unimplemented!()
    }
    fn get_process_metrics(&self, topology: &Topology, data: &Vec<Metric>) {
        unimplemented!()
    }
    //fn manage_metric<T>(&self, client: T, data: &Vec<Metric>);
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
