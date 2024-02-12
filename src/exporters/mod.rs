//! # Exporters: to make data accessible to monitoring toolchains
//!
//! `Exporter` is the root for all exporters. It defines the [Exporter] trait
//! needed to implement an exporter.
#[cfg(feature = "json")]
pub mod json;
#[cfg(feature = "prometheus")]
pub mod prometheus;
#[cfg(feature = "prometheuspush")]
pub mod prometheuspush;
#[cfg(target_os = "linux")]
pub mod qemu;
#[cfg(feature = "riemann")]
pub mod riemann;
pub mod stdout;
pub mod utils;
#[cfg(feature = "warpten")]
pub mod warpten;
use crate::sensors::{
    utils::{current_system_time_since_epoch, IProcess},
    RecordGenerator, Topology,
};
use chrono::Utc;
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;
use utils::get_scaphandre_version;
#[cfg(feature = "containers")]
use {
    docker_sync::{container::Container, Docker},
    k8s_sync::kubernetes::Kubernetes,
    k8s_sync::Pod,
    ordered_float::*,
    regex::Regex,
    utils::{get_docker_client, get_kubernetes_client},
};

/// General metric definition.
#[derive(Debug)]
pub struct Metric {
    /// `name` is the metric name, it will be used as service field for Riemann.
    name: String, // Will be used as service for Riemann
    /// `metric_type` mostly used by Prometheus, define is it is a gauge, counter...
    metric_type: String,
    /// `ttl` time to live for this metric used by Riemann.
    #[allow(dead_code)]
    ttl: f32,
    /// `hostname` host that provides the metric.
    hostname: String,
    /// `state` used by Riemann, define a state like Ok or Ko regarding this metric.
    #[allow(dead_code)]
    state: String,
    /// `tags` used by Riemann, tags attached to the metric.
    #[allow(dead_code)]
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
    /// `timestamp` is the timestamp of the moment of the data measurement, stored as a Duration
    timestamp: Duration,
}

enum MetricValueType {
    // IntSigned(i64),
    // Float(f32),
    Text(String),
    //FloatDouble(f64),
    IntUnsigned(u64),
}

impl fmt::Display for MetricValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            // MetricValueType::IntSigned(value) => write!(f, "{}", value),
            // MetricValueType::Float(value) => write!(f, "{}", value),
            MetricValueType::Text(text) => write!(f, "{text}"),
            //MetricValueType::FloatDouble(value) => write!(f, "{value}"),
            MetricValueType::IntUnsigned(value) => write!(f, "{value}"),
        }
    }
}

impl fmt::Debug for MetricValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            // MetricValueType::IntSigned(value) => write!(f, "{}", value),
            // MetricValueType::Float(value) => write!(f, "{}", value),
            MetricValueType::Text(text) => write!(f, "{text}"),
            //MetricValueType::FloatDouble(value) => write!(f, "{value}"),
            MetricValueType::IntUnsigned(value) => write!(f, "{value}"),
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
    /// Runs the exporter.
    fn run(&mut self);

    /// The name of the kind of the exporter, for example "json".
    fn kind(&self) -> &str;
}

/// MetricGenerator is an exporter helper structure to collect Scaphandre metrics.
/// The goal is to provide a standard Vec\<Metric\> that can be used by exporters
/// to avoid code duplication.
pub struct MetricGenerator {
    /// `data` will be used to store the metrics retrieved.
    data: Vec<Metric>,
    /// `topology` is the system physical layout retrieve via the sensors crate with
    /// associated metrics.
    topology: Topology,
    /// `hostname` is the system name where the metrics belongs.
    hostname: String,
    /// Tells MetricGenerator if it has to watch for qemu virtual machines.
    #[cfg(target_os = "linux")]
    qemu: bool,
    /// Tells MetricGenerator if it has to watch for containers.
    #[cfg(feature = "containers")]
    watch_containers: bool,
    ///
    #[cfg(feature = "containers")]
    containers_last_check: String,
    /// `containers` contains the containers descriptions when --containers is true
    #[cfg(feature = "containers")]
    containers: Vec<Container>,
    /// docker_version contains the version number of local docker daemon
    #[cfg(feature = "containers")]
    docker_version: String,
    /// docker_client holds the opened docker socket
    #[cfg(feature = "containers")]
    docker_client: Option<Docker>,
    /// watch Docker
    #[cfg(feature = "containers")]
    watch_docker: bool,
    /// watch Kubernetes
    #[cfg(feature = "containers")]
    watch_kubernetes: bool,
    /// kubernetes socket
    #[cfg(feature = "containers")]
    kubernetes_client: Option<Kubernetes>,
    /// Kubernetes pods
    #[cfg(feature = "containers")]
    pods: Vec<Pod>,
    ///
    #[cfg(feature = "containers")]
    pods_last_check: String,
}

/// This is not mandatory to use MetricGenerator methods. Exporter can use dedicated
/// code into the [Exporter] run() method to collect metrics. However it is advised
/// to use the following methods to avoid discrepancies between exporters.
impl MetricGenerator {
    /// Returns a MetricGenerator instance that will host metrics.

    pub fn new(
        topology: Topology,
        hostname: String,
        _qemu: bool,
        _watch_containers: bool,
    ) -> MetricGenerator {
        let data = Vec::new();
        #[cfg(feature = "containers")]
        {
            let containers = vec![];
            let pods = vec![];
            let docker_version = String::from("");
            let mut docker_client = None;
            let mut kubernetes_client = None;
            let mut container_runtime = false;
            if _watch_containers {
                match get_docker_client() {
                    Ok(docker) => {
                        docker_client = Some(docker);
                        container_runtime = true;
                    }
                    Err(err) => {
                        info!("Couldn't connect to docker socket. Error: {}", err);
                    }
                }
                if let Ok(kubernetes) = get_kubernetes_client() {
                    kubernetes_client = Some(kubernetes);
                    container_runtime = true;
                } else {
                    info!("Couldn't connect to kubernetes API.");
                }
                if !container_runtime {
                    warn!("--containers was used but scaphandre couldn't connect to any container runtime.");
                }
            }
            MetricGenerator {
                data,
                topology,
                hostname,
                containers,
                #[cfg(target_os = "linux")]
                qemu: _qemu,
                containers_last_check: String::from(""),
                docker_version,
                docker_client,
                watch_containers: _watch_containers,
                watch_docker: true,
                kubernetes_client,
                watch_kubernetes: true,
                pods,
                pods_last_check: String::from(""),
                //kubernetes_version,
            }
        }
        #[cfg(not(feature = "containers"))]
        MetricGenerator {
            data,
            topology,
            hostname,
            #[cfg(target_os = "linux")]
            qemu: _qemu,
        }
    }

    #[cfg(feature = "containers")]
    pub fn get_processes_filtered_by_container_name(
        &self,
        container_regex: &Regex,
    ) -> Vec<(IProcess, f64)> {
        let mut consumers: Vec<(IProcess, OrderedFloat<f64>)> = vec![];
        for p in &self.topology.proc_tracker.procs {
            if p.len() > 1 {
                let diff = self.topology.proc_tracker.get_cpu_usage_percentage(
                    p.first().unwrap().process.pid as _,
                    self.topology.proc_tracker.nb_cores,
                );
                let p_record = p.last().unwrap();
                let container_description = self
                    .topology
                    .proc_tracker
                    .get_process_container_description(
                        p_record.process.pid,
                        &self.containers,
                        self.docker_version.clone(),
                        &self.pods,
                    );
                if let Some(name) = container_description.get("container_names") {
                    if container_regex.is_match(name) {
                        consumers.push((p_record.process.clone(), OrderedFloat(diff as f64)));
                        consumers.sort_by(|x, y| y.1.cmp(&x.1));
                    }
                }
                //if container_regex.is_match(process_exe.to_str().unwrap_or_default()) {
                //    consumers.push((p_record.process.clone(), OrderedFloat(diff as f64)));
                //    consumers.sort_by(|x, y| y.1.cmp(&x.1));
                //} else if container_regex.is_match(&process_cmdline.concat()) {
                //    consumers.push((p_record.process.clone(), OrderedFloat(diff as f64)));
                //    consumers.sort_by(|x, y| y.1.cmp(&x.1));
                //}
            }
        }
        let mut result: Vec<(IProcess, f64)> = vec![];
        for (p, f) in consumers {
            result.push((p, f.into_inner()));
        }
        result
    }

    /// Generate all scaphandre internal metrics.
    fn gen_self_metrics(&mut self) {
        let myself = IProcess::myself(self.topology.get_proc_tracker()).unwrap();

        let default_timestamp = current_system_time_since_epoch();
        self.data.push(Metric {
            name: String::from("scaph_self_version"),
            metric_type: String::from("gauge"),
            ttl: 60.0,
            hostname: self.hostname.clone(),
            state: String::from("ok"),
            timestamp: default_timestamp,
            tags: vec!["scaphandre".to_string()],
            attributes: HashMap::new(),
            description: String::from("Version number of scaphandre represented as a float."),
            metric_value: MetricValueType::Text(get_scaphandre_version()),
        });

        if let Some(metric_value) = self.topology.get_process_cpu_usage_percentage(myself.pid) {
            self.data.push(Metric {
                name: String::from("scaph_self_cpu_usage_percent"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                timestamp: metric_value.timestamp,
                hostname: self.hostname.clone(),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                description: format!("CPU time consumed by scaphandre, as {}", metric_value.unit),
                metric_value: MetricValueType::Text(metric_value.value),
            });
        }

        if let Some(metric_value) = self.topology.get_process_memory_virtual_bytes(myself.pid) {
            self.data.push(Metric {
                name: String::from("scaph_self_memory_virtual_bytes"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                timestamp: default_timestamp,
                hostname: self.hostname.clone(),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                description: format!("Total program size, measured in {}.", metric_value.unit),
                metric_value: MetricValueType::IntUnsigned(
                    metric_value.value.parse::<u64>().unwrap(),
                ),
            });
        }

        if let Some(metric_value) = self.topology.get_process_memory_bytes(myself.pid) {
            self.data.push(Metric {
                name: String::from("scaph_self_memory_bytes"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                hostname: self.hostname.clone(),
                state: String::from("ok"),
                timestamp: default_timestamp,
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                description: String::from("Resident set size, measured in bytes."),
                metric_value: MetricValueType::IntUnsigned(
                    metric_value.value.parse::<u64>().unwrap(),
                ),
            });
        }

        let topo_stat_buffer_len = self.topology.stat_buffer.len();
        let topo_record_buffer_len = self.topology.record_buffer.len();
        let topo_procs_len = self.topology.proc_tracker.procs.len();

        self.data.push(Metric {
            name: String::from("scaph_self_topo_stats_nb"),
            metric_type: String::from("gauge"),
            ttl: 60.0,
            timestamp: default_timestamp,
            hostname: self.hostname.clone(),
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
            timestamp: default_timestamp,
            hostname: self.hostname.clone(),
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
            timestamp: default_timestamp,
            hostname: self.hostname.clone(),
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
                timestamp: default_timestamp,
                hostname: self.hostname.clone(),
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
                timestamp: default_timestamp,
                hostname: self.hostname.clone(),
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
                    timestamp: default_timestamp,
                    hostname: self.hostname.clone(),
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
            let mut attributes = HashMap::new();
            if self.topology._sensor_data.contains_key("psys") {
                attributes.insert(
                    String::from("value_source"),
                    String::from("powercap_rapl_psys"),
                );
            } else if self.topology._sensor_data.contains_key("source_file") {
                attributes.insert(
                    String::from("value_source"),
                    String::from("powercap_rapl_pkg"),
                );
            } else if self.topology._sensor_data.contains_key("DRIVER_NAME") {
                attributes.insert(
                    String::from("value_source"),
                    String::from("scaphandredrv_rapl_pkg"),
                );
            }

            self.data.push(Metric {
                    name: String::from("scaph_host_energy_microjoules"),
                    metric_type: String::from("counter"),
                    ttl: 60.0,
                    timestamp: record.timestamp,
                    hostname: self.hostname.clone(),
                    state: String::from("ok"),
                    tags: vec!["scaphandre".to_string()],
                    attributes: attributes.clone(),
                    description: String::from(
                        "Energy measurement for the whole host, as extracted from the sensor, in microjoules.",
                    ),
                    metric_value: MetricValueType::Text(host_energy_microjoules),
                });

            if let Some(power) = self.topology.get_records_diff_power_microwatts() {
                self.data.push(Metric {
                    name: String::from("scaph_host_power_microwatts"),
                    metric_type: String::from("gauge"),
                    ttl: 60.0,
                    timestamp: power.timestamp,
                    hostname: self.hostname.clone(),
                    state: String::from("ok"),
                    tags: vec!["scaphandre".to_string()],
                    attributes,
                    description: String::from("Power measurement on the whole host, in microwatts"),
                    metric_value: MetricValueType::Text(power.value),
                });
            }
        }
        if let Some(metric_value) = self.topology.get_load_avg() {
            self.data.push(Metric {
                name: String::from("scaph_host_load_avg_one"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                timestamp: metric_value[0].timestamp,
                hostname: self.hostname.clone(),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                description: String::from("Load average on 1 minute."),
                metric_value: MetricValueType::Text(metric_value[0].value.clone()),
            });
            self.data.push(Metric {
                name: String::from("scaph_host_load_avg_five"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                timestamp: metric_value[1].timestamp,
                hostname: self.hostname.clone(),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                description: String::from("Load average on 5 minutes."),
                metric_value: MetricValueType::Text(metric_value[1].value.clone()),
            });
            self.data.push(Metric {
                name: String::from("scaph_host_load_avg_fifteen"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                timestamp: metric_value[2].timestamp,
                hostname: self.hostname.clone(),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                description: String::from("Load average on 15 minutes."),
                metric_value: MetricValueType::Text(metric_value[2].value.clone()),
            });
        }
        let freq = self.topology.get_cpu_frequency();
        self.data.push(Metric {
            name: String::from("scaph_host_cpu_frequency"),
            metric_type: String::from("gauge"),
            ttl: 60.0,
            timestamp: freq.timestamp,
            hostname: self.hostname.clone(),
            state: String::from("ok"),
            tags: vec!["scaphandre".to_string()],
            attributes: HashMap::new(),
            description: format!("Global frequency of all the cpus. In {}", freq.unit),
            metric_value: MetricValueType::Text(freq.value),
        });
        for (metric_name, metric) in self.topology.get_disks() {
            info!("pushing disk metric to data : {}", metric_name);
            self.data.push(Metric {
                name: metric_name,
                metric_type: String::from("gauge"),
                ttl: 60.0,
                timestamp: metric.2.timestamp,
                hostname: self.hostname.clone(),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: metric.1,
                description: metric.0,
                metric_value: MetricValueType::Text(metric.2.value),
            });
        }

        let ram_attributes = HashMap::new();
        let metric_value = self.topology.get_total_memory_bytes();
        self.data.push(Metric {
            name: String::from("scaph_host_memory_total_bytes"),
            metric_type: String::from("gauge"),
            ttl: 60.0,
            timestamp: metric_value.timestamp,
            hostname: self.hostname.clone(),
            state: String::from("ok"),
            tags: vec!["scaphandre".to_string()],
            attributes: ram_attributes.clone(),
            description: String::from("Random Access Memory installed on the host, in bytes."),
            metric_value: MetricValueType::Text(metric_value.value),
        });
        let metric_value = self.topology.get_available_memory_bytes();
        self.data.push(Metric {
            name: String::from("scaph_host_memory_available_bytes"),
            metric_type: String::from("gauge"),
            ttl: 60.0,
            timestamp: metric_value.timestamp,
            hostname: self.hostname.clone(),
            state: String::from("ok"),
            tags: vec!["scaphandre".to_string()],
            attributes: ram_attributes.clone(),
            description: String::from(
                "Random Access Memory available to be re-used on the host, in bytes.",
            ),
            metric_value: MetricValueType::Text(metric_value.value),
        });
        let metric_value = self.topology.get_free_memory_bytes();
        self.data.push(Metric {
            name: String::from("scaph_host_memory_free_bytes"),
            metric_type: String::from("gauge"),
            ttl: 60.0,
            timestamp: metric_value.timestamp,
            hostname: self.hostname.clone(),
            state: String::from("ok"),
            tags: vec!["scaphandre".to_string()],
            attributes: ram_attributes.clone(),
            description: String::from(
                "Random Access Memory free to be used (not reused) on the host, in bytes.",
            ),
            metric_value: MetricValueType::Text(metric_value.value),
        });
        let metric_value = self.topology.get_free_swap_bytes();
        self.data.push(Metric {
            name: String::from("scaph_host_swap_free_bytes"),
            metric_type: String::from("gauge"),
            ttl: 60.0,
            timestamp: metric_value.timestamp,
            hostname: self.hostname.clone(),
            state: String::from("ok"),
            tags: vec!["scaphandre".to_string()],
            attributes: ram_attributes.clone(),
            description: String::from("Swap space free to be used on the host, in bytes."),
            metric_value: MetricValueType::Text(metric_value.value),
        });
        let metric_value = self.topology.get_total_swap_bytes();
        self.data.push(Metric {
            name: String::from("scaph_host_swap_total_bytes"),
            metric_type: String::from("gauge"),
            ttl: 60.0,
            timestamp: metric_value.timestamp,
            hostname: self.hostname.clone(),
            state: String::from("ok"),
            tags: vec!["scaphandre".to_string()],
            attributes: ram_attributes,
            description: String::from("Total swap space on the host, in bytes."),
            metric_value: MetricValueType::Text(metric_value.value),
        });
    }

    /// Generate socket metrics.
    fn gen_socket_metrics(&mut self) {
        let sockets = self.topology.get_sockets_passive();
        for socket in sockets {
            let records = socket.get_records_passive();
            let mut attributes = HashMap::new();
            attributes.insert("socket_id".to_string(), socket.id.to_string());
            if !records.is_empty() {
                let metric = records.last().unwrap();
                let metric_value = metric.value.clone();
                let metric_timestamp = metric.timestamp;

                self.data.push(Metric {
                    name: String::from("scaph_socket_energy_microjoules"),
                    metric_type: String::from("counter"),
                    ttl: 60.0,
                    timestamp: metric_timestamp,
                    hostname: self.hostname.clone(),
                    state: String::from("ok"),
                    tags: vec!["scaphandre".to_string()],
                    attributes: attributes.clone(),
                    description: String::from("Socket related energy measurement in microjoules."),
                    metric_value: MetricValueType::Text(metric_value.clone()),
                });

                if let Some(power) = socket.get_records_diff_power_microwatts() {
                    let socket_power_microwatts = &power.value;

                    self.data.push(Metric {
                        name: String::from("scaph_socket_power_microwatts"),
                        metric_type: String::from("gauge"),
                        ttl: 60.0,
                        timestamp: power.timestamp,
                        hostname: self.hostname.clone(),
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
            if let Some(mmio) = socket.get_rapl_mmio_energy_microjoules() {
                self.data.push(Metric {
                    name: String::from("scaph_socket_rapl_mmio_energy_microjoules"),
                    metric_type: String::from("counter"),
                    ttl: 60.0,
                    timestamp: mmio.timestamp,
                    hostname: self.hostname.clone(),
                    state: String::from("ok"),
                    tags: vec!["scaphandre".to_string()],
                    attributes: attributes.clone(),
                    description: format!(
                        "Energy counter from RAPL mmio interface for Package-0 of CPU socket {}",
                        socket.id
                    ),
                    metric_value: MetricValueType::Text(mmio.value),
                });
            }
            for domain in socket.get_domains_passive() {
                let records = domain.get_records_passive();
                if !records.is_empty() {
                    let metric = records.last().unwrap();
                    let metric_value = metric.value.clone();
                    let metric_timestamp = metric.timestamp;

                    let mut attributes = HashMap::new();
                    attributes.insert("domain_name".to_string(), domain.name.clone());
                    attributes.insert("domain_id".to_string(), domain.id.to_string());
                    attributes.insert("socket_id".to_string(), socket.id.to_string());

                    self.data.push(Metric {
                        name: String::from("scaph_domain_energy_microjoules"),
                        metric_type: String::from("counter"),
                        ttl: 60.0,
                        hostname: self.hostname.clone(),
                        timestamp: metric_timestamp,
                        state: String::from("ok"),
                        tags: vec!["scaphandre".to_string()],
                        attributes: attributes.clone(),
                        description: String::from(
                            "Domain related energy measurement in microjoules.",
                        ),
                        metric_value: MetricValueType::Text(metric_value.clone()),
                    });

                    if let Some(power) = domain.get_records_diff_power_microwatts() {
                        let domain_power_microwatts = &power.value;
                        self.data.push(Metric {
                            name: String::from("scaph_domain_power_microwatts"),
                            metric_type: String::from("gauge"),
                            ttl: 60.0,
                            hostname: self.hostname.clone(),
                            timestamp: power.timestamp,
                            state: String::from("ok"),
                            tags: vec!["scaphandre".to_string()],
                            attributes: attributes.clone(),
                            description: String::from(
                                "Power measurement relative to a RAPL Domain, in microwatts",
                            ),
                            metric_value: MetricValueType::Text(domain_power_microwatts.clone()),
                        });
                    }
                    let mut mmio_attributes = attributes.clone();
                    mmio_attributes.insert(
                        String::from("value_source"),
                        String::from("powercap_rapl_mmio"),
                    );
                    if let Some(mmio) = domain.get_rapl_mmio_energy_microjoules() {
                        self.data.push(Metric {
                            name: String::from("scaph_domain_rapl_mmio_energy_microjoules"),
                            metric_type: String::from("counter"),
                            ttl: 60.0,
                            timestamp: mmio.timestamp,
                            hostname: self.hostname.clone(),
                            state: String::from("ok"),
                            tags: vec!["scaphandre".to_string()],
                            attributes: mmio_attributes,
                            description: format!(
                                "Energy counter from RAPL mmio interface for the {} domain, socket {}.", domain.name, socket.id
                            ),
                            metric_value: MetricValueType::Text(mmio.value),
                        });
                    }
                }
            }
        }
    }

    /// Generate system metrics.
    fn gen_system_metrics(&mut self) {
        let default_timestamp = current_system_time_since_epoch();
        if let Some(metric_value) = self.topology.read_nb_process_total_count() {
            self.data.push(Metric {
                name: String::from("scaph_forks_since_boot_total"),
                metric_type: String::from("counter"),
                ttl: 60.0,
                timestamp:  default_timestamp,
                hostname: self.hostname.clone(),
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
                timestamp: default_timestamp,
                hostname: self.hostname.clone(),
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
                timestamp: default_timestamp,
                hostname: self.hostname.clone(),
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
                timestamp: default_timestamp,
                hostname: self.hostname.clone(),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                description: String::from("Number of context switches since boot."),
                metric_value: MetricValueType::IntUnsigned(metric_value),
            });
        }
    }

    /// If *self.watch_docker* is true and *self.docker_client* is Some
    /// gets the list of docker containers running on the machine, thanks
    /// to *self.docker_client*. Stores the resulting vector as *self.containers*.
    /// Updates *self.containers_last_check* to the current timestamp, if the
    /// operation is successful.
    #[cfg(feature = "containers")]
    fn gen_docker_containers_basic_metadata(&mut self) {
        if self.watch_docker && self.docker_client.is_some() {
            if let Some(docker) = self.docker_client.as_mut() {
                if let Ok(containers_result) = docker.get_containers(false) {
                    self.containers = containers_result;
                    self.containers_last_check =
                        current_system_time_since_epoch().as_secs().to_string();
                }
            } else {
                debug!("Docker socket is None.");
            }
        }
    }

    /// If *self.watch_kubernetes* is true,
    /// queries the local kubernetes API (if this is a kubernetes cluster node)
    /// and retrieves the list of pods running on this node, thanks to *self.kubernetes_client*.
    /// Stores the result as *self.pods* and updates *self.pods_last_check* if the operation is successfull.
    #[cfg(feature = "containers")]
    fn gen_kubernetes_pods_basic_metadata(&mut self) {
        if self.watch_kubernetes {
            if let Some(kubernetes) = self.kubernetes_client.as_mut() {
                if let Ok(pods_result) = kubernetes.list_pods("".to_string()) {
                    self.pods = pods_result;
                    debug!("Found {} pods", &self.pods.len());
                } else {
                    debug!("Failed getting pods list, despite client seems ok.");
                }
            } else {
                debug!("Kubernetes socket is not some.");
            }
            self.pods_last_check = current_system_time_since_epoch().as_secs().to_string();
        }
    }

    /// Generate process metrics.
    fn gen_process_metrics(&mut self) {
        trace!("In gen_process_metrics.");
        #[cfg(feature = "containers")]
        if self.watch_containers {
            let now = current_system_time_since_epoch().as_secs().to_string();
            if self.watch_docker && self.docker_client.is_some() {
                let last_check = self.containers_last_check.clone();
                if last_check.is_empty() {
                    match self.docker_client.as_mut().unwrap().get_version() {
                        Ok(version_response) => {
                            self.docker_version = String::from(version_response.Version.as_str());
                            self.gen_docker_containers_basic_metadata();
                        }
                        Err(error) => {
                            info!("Couldn't query the docker socket: {}", error);
                            self.watch_docker = false;
                        }
                    }
                } else {
                    match self
                        .docker_client
                        .as_mut()
                        .unwrap()
                        .get_events(Some(last_check), Some(now.clone()))
                    {
                        Ok(events) => {
                            if !events.is_empty() {
                                self.gen_docker_containers_basic_metadata();
                            }
                        }
                        Err(err) => debug!("couldn't get docker events - {:?} - {}", err, err),
                    }
                }
                self.containers_last_check =
                    current_system_time_since_epoch().as_secs().to_string();
            }
            if self.watch_kubernetes && self.kubernetes_client.is_some() {
                if self.pods_last_check.is_empty() {
                    self.gen_kubernetes_pods_basic_metadata();
                    debug!("First check done on pods.");
                }
                let last_check = self.pods_last_check.clone();
                if (now.parse::<i32>().unwrap() - last_check.parse::<i32>().unwrap()) > 20 {
                    debug!(
                        "Just refreshed pod list ! last: {} now: {}, diff: {}",
                        last_check,
                        now,
                        (now.parse::<i32>().unwrap() - last_check.parse::<i32>().unwrap())
                    );
                    self.gen_kubernetes_pods_basic_metadata();
                }
            }
        }
        debug!("Before loop.");

        for pid in self.topology.proc_tracker.get_alive_pids() {
            let exe = self.topology.proc_tracker.get_process_name(pid);
            let cmdline = self.topology.proc_tracker.get_process_cmdline(pid);

            let mut attributes = HashMap::new();
            debug!("Working on {}: {}", pid, exe);

            #[cfg(feature = "containers")]
            if self.watch_containers && (!self.containers.is_empty() || !self.pods.is_empty()) {
                let container_data = self
                    .topology
                    .proc_tracker
                    .get_process_container_description(
                        pid,
                        &self.containers,
                        self.docker_version.clone(),
                        &self.pods,
                        //self.kubernetes_version.clone(),
                    );

                if !container_data.is_empty() {
                    for (k, v) in container_data.iter() {
                        attributes.insert(String::from(k), String::from(v));
                    }
                }
            }

            attributes.insert("pid".to_string(), pid.to_string());

            attributes.insert("exe".to_string(), exe.clone());

            if let Some(cmdline_str) = cmdline {
                attributes.insert("cmdline".to_string(), utils::filter_cmdline(&cmdline_str));

                #[cfg(target_os = "linux")]
                if self.qemu {
                    if let Some(vmname) = utils::filter_qemu_cmdline(&cmdline_str) {
                        attributes.insert("vmname".to_string(), vmname);
                    }
                }
            }

            if let Some(metrics) = self.topology.get_all_per_process(pid) {
                for (k, v) in metrics {
                    self.data.push(Metric {
                        name: k,
                        metric_type: String::from("gauge"),
                        ttl: 60.0,
                        timestamp: v.1.timestamp,
                        hostname: self.hostname.clone(),
                        state: String::from("ok"),
                        tags: vec!["scaphandre".to_string()],
                        attributes: attributes.clone(),
                        description: v.0,
                        metric_value: MetricValueType::Text(v.1.value),
                    })
                }
            }
        }
    }

    /// Generate all metrics provided by Scaphandre agent.
    fn gen_all_metrics(&mut self) {
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
        self.gen_process_metrics();
        trace!("self_metrics: {:#?}", self.data);
    }

    pub fn pop_metrics(&mut self) -> Vec<Metric> {
        let mut res = vec![];
        while !&self.data.is_empty() {
            res.push(self.data.pop().unwrap())
        }
        res
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
