//! # Exporters: to make data accessible to monitoring toolchains
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
use crate::sensors::{utils::current_system_time_since_epoch, RecordGenerator, Topology};
use chrono::Utc;
use clap::ArgMatches;
use docker_sync::{container::Container, Docker};
use k8s_sync::kubernetes::Kubernetes;
use k8s_sync::Pod;
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;
use utils::{get_docker_client, get_kubernetes_client, get_scaphandre_version};

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
    /// `timestamp` is the timestamp of the moment of the data measurement, stored as a Duration
    timestamp: Duration,
}

#[derive(Clone)]
enum MetricValueType {
    // IntSigned(i64),
    // Float(f32),
    Text(String),
    FloatDouble(f64),
    IntUnsigned(u64),
}

impl fmt::Display for MetricValueType {
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
struct MetricGenerator {
    /// `data` will be used to store the metrics retrieved.
    data: Vec<Metric>,
    /// `topology` is the system physical layout retrieve via the sensors crate with
    /// associated metrics.
    topology: Topology,
    /// `hostname` is the system name where the metrics belongs.
    hostname: String,
    /// Tells MetricGenerator if it has to watch for qemu virtual machines.
    qemu: bool,
    /// Tells MetricGenerator if it has to watch for containers.
    watch_containers: bool,
    ///
    containers_last_check: String,
    /// `containers` contains the containers descriptions when --containers is true
    containers: Vec<Container>,
    /// docker_version contains the version number of local docker daemon
    docker_version: String,
    /// docker_client holds the opened docker socket
    docker_client: Option<Docker>,
    /// watch Docker
    watch_docker: bool,
    /// watch Kubernetes
    watch_kubernetes: bool,
    /// kubernetes socket
    kubernetes_client: Option<Kubernetes>,
    /// Kubernetes pods
    pods: Vec<Pod>,
    ///
    pods_last_check: String,
    // kubernetes cluster version
    //kubernetes_version: String,
}

/// This is not mandatory to use MetricGenerator methods. Exporter can use dedicated
/// code into the [Exporter] run() method to collect metrics. However it is advised
/// to use the following methods to avoid discrepancies between exporters.
impl MetricGenerator {
    /// Returns a MetricGenerator instance that will host metrics.

    fn new(
        topology: Topology,
        hostname: String,
        qemu: bool,
        watch_containers: bool,
    ) -> MetricGenerator {
        let data = Vec::new();
        let containers = vec![];
        let pods = vec![];
        let docker_version = String::from("");
        let mut docker_client = None;
        //let kubernetes_version = String::from("");
        let mut kubernetes_client = None;
        if watch_containers {
            let mut container_runtime = false;
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
            qemu,
            containers_last_check: String::from(""),
            docker_version,
            docker_client,
            watch_containers,
            watch_docker: true,
            kubernetes_client,
            watch_kubernetes: true,
            pods,
            pods_last_check: String::from(""),
            //kubernetes_version,
        }
    }

    /// Generate all scaphandre internal metrics.
    fn gen_self_metrics(&mut self) {
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

        if let Some(metric_value) = self
            .topology
            .get_process_cpu_consumption_percentage(procfs::process::Process::myself().unwrap().pid)
        {
            self.data.push(Metric {
                name: String::from("scaph_self_cpu_usage_percent"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                timestamp: metric_value.timestamp,
                hostname: self.hostname.clone(),
                state: String::from("ok"),
                tags: vec!["scaphandre".to_string()],
                attributes: HashMap::new(),
                description: String::from("CPU % consumed by scaphandre."),
                metric_value: MetricValueType::FloatDouble(
                    metric_value.value.parse::<f64>().unwrap(),
                ),
            });
        }

        if let Ok(metric_value) = procfs::process::Process::myself().unwrap().statm() {
            let value = metric_value.size * procfs::page_size().unwrap() as u64;
            self.data.push(Metric {
                name: String::from("scaph_self_mem_total_program_size"),
                metric_type: String::from("gauge"),
                ttl: 60.0,
                timestamp: default_timestamp,
                hostname: self.hostname.clone(),
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
                hostname: self.hostname.clone(),
                state: String::from("ok"),
                timestamp: default_timestamp,
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
                timestamp: default_timestamp,
                hostname: self.hostname.clone(),
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

            self.data.push(Metric {
                    name: String::from("scaph_host_energy_microjoules"),
                    metric_type: String::from("counter"),
                    ttl: 60.0,
                    timestamp: record.timestamp,
                    hostname: self.hostname.clone(),
                    state: String::from("ok"),
                    tags: vec!["scaphandre".to_string()],
                    attributes: HashMap::new(),
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
                let metric = records.last().unwrap();
                let metric_value = metric.value.clone();
                let metric_timestamp = metric.timestamp;

                let mut attributes = HashMap::new();
                attributes.insert("socket_id".to_string(), socket.id.to_string());

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
                metric_value: MetricValueType::IntUnsigned(metric_value as u64),
            });
        }
    }

    /// If *self.watch_docker* is true and *self.docker_client* is Some
    /// gets the list of docker containers running on the machine, thanks
    /// to *self.docker_client*. Stores the resulting vector as *self.containers*.
    /// Updates *self.containers_last_check* to the current timestamp, if the
    /// operation is successful.
    fn gen_docker_containers_basic_metadata(&mut self) {
        if self.watch_docker && self.docker_client.is_some() {
            if let Some(docker) = self.docker_client.as_mut() {
                if let Ok(containers_result) = docker.get_containers(false) {
                    self.containers = containers_result;
                    self.containers_last_check =
                        current_system_time_since_epoch().as_secs().to_string();
                }
            } else {
                info!("Docker socket is None.");
            }
        }
    }

    /// If *self.watch_kubernetes* is true,
    /// queries the local kubernetes API (if this is a kubernetes cluster node)
    /// and retrieves the list of pods running on this node, thanks to *self.kubernetes_client*.
    /// Stores the result as *self.pods* and updates *self.pods_last_check* if the operation is successfull.
    fn gen_kubernetes_pods_basic_metadata(&mut self) {
        if self.watch_kubernetes {
            if let Some(kubernetes) = self.kubernetes_client.as_mut() {
                if let Ok(pods_result) = kubernetes.list_pods("".to_string()) {
                    self.pods = pods_result;
                    debug!("Found {} pods", &self.pods.len());
                } else {
                    info!("Failed getting pods list, despite client seems ok.");
                }
            } else {
                debug!("Kubernetes socket is not some.");
            }
            self.pods_last_check = current_system_time_since_epoch().as_secs().to_string();
        }
    }

    /// Generate process metrics.
    fn gen_process_metrics(&mut self) {
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
                            } else {
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
                    info!("First check done on pods.");
                }
                let last_check = self.pods_last_check.clone();
                if (now.parse::<i32>().unwrap() - last_check.parse::<i32>().unwrap()) > 20 {
                    info!(
                        "Just refreshed pod list ! last: {} now: {}, diff: {}",
                        last_check,
                        now,
                        (now.parse::<i32>().unwrap() - last_check.parse::<i32>().unwrap())
                    );
                    self.gen_kubernetes_pods_basic_metadata();
                }
            }
        }

        for pid in self.topology.proc_tracker.get_alive_pids() {
            let exe = self.topology.proc_tracker.get_process_name(pid);
            let cmdline = self.topology.proc_tracker.get_process_cmdline(pid);

            let mut attributes = HashMap::new();

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
                attributes.insert("cmdline".to_string(), cmdline_str.replace('\"', "\\\""));

                if self.qemu {
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
                    timestamp: power.timestamp,
                    hostname: self.hostname.clone(),
                    state: String::from("ok"),
                    tags: vec!["scaphandre".to_string()],
                    attributes,
                    description: String::from("Power consumption due to the process, measured on at the topology level, in microwatts"),
                    metric_value: MetricValueType::Text(power.value),
                });
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
        debug!("self_metrics: {:#?}", self.data);
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
