use crate::exporters::*;
use crate::sensors::{RecordGenerator, Sensor};
use clap::crate_version;
use riemann_client::proto::Event;
use riemann_client::Client;
use std::collections::HashMap;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Riemann server default ipv4/ipv6 address
const DEFAULT_IP_ADDRESS: &str = "localhost";

/// Riemann server default port
const DEFAULT_PORT: &str = "5555";

/// Metric trait to deal with metric types
trait Metric {
    fn set_metric(self, event: &mut Event);
}

impl Metric for usize {
    fn set_metric(self, event: &mut Event) {
        event.set_metric_sint64(self as i64);
    }
}

impl Metric for u64 {
    fn set_metric(self, event: &mut Event) {
        event.set_metric_sint64(self as i64);
    }
}

impl Metric for u32 {
    fn set_metric(self, event: &mut Event) {
        event.set_metric_sint64(self as i64);
    }
}

impl Metric for isize {
    fn set_metric(self, event: &mut Event) {
        event.set_metric_sint64(self as i64);
    }
}

impl Metric for i64 {
    fn set_metric(self, event: &mut Event) {
        event.set_metric_sint64(self);
    }
}

impl Metric for i32 {
    fn set_metric(self, event: &mut Event) {
        event.set_metric_sint64(self as i64);
    }
}

impl Metric for f32 {
    fn set_metric(self, event: &mut Event) {
        event.set_metric_f(self);
    }
}

impl Metric for f64 {
    fn set_metric(self, event: &mut Event) {
        event.set_metric_d(self);
    }
}

impl Metric for &str {
    fn set_metric(self, event: &mut Event) {
        let metric = self.replace(",", ".");
        if metric.contains('.') {
            event.set_metric_d(metric.parse::<f64>().expect("Cannot parse metric"));
        } else {
            event.set_metric_sint64(metric.parse::<i64>().expect("Cannot parse metric"));
        }
    }
}

impl Metric for String {
    fn set_metric(self, event: &mut Event) {
        let metric = self.replace(",", ".");
        if metric.contains('.') {
            event.set_metric_d(metric.parse::<f64>().expect("Cannot parse metric"));
        } else {
            event.set_metric_sint64(metric.parse::<i64>().expect("Cannot parse metric"));
        }
    }
}

/// Riemann client
struct Riemann {
    client: Client,
}

impl Riemann {
    fn new(address: &str, port: &str) -> Riemann {
        let address = String::from(address);
        let port = port.parse::<u16>().expect("Fail parsing port number");
        let client = Client::connect(&(address, port)).expect("Fail to connect to Riemann server");
        Riemann { client }
    }

    #[allow(clippy::too_many_arguments)]
    fn send_metric<T: Metric>(
        &mut self,
        ttl: f32,
        hostname: &str,
        service: &str,
        state: &str,
        tag: Vec<String>,
        description: &str,
        metric: T,
    ) {
        let mut event = Event::new();
        event.set_time(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        );
        event.set_ttl(ttl);
        event.set_host(hostname.to_string());
        event.set_service(service.to_string());
        event.set_state(state.to_string());
        event.set_tags(protobuf::RepeatedField::from_vec(tag));
        event.set_description(description.to_string());
        metric.set_metric(&mut event);
        self.client
            .event(event)
            .expect("Fail to send metric to Riemann");
    }
}

/// Exporter sends metrics to a Riemann server
pub struct RiemannExporter {
    /// Sensor instance that is used to generate the Topology and
    /// thus get power consumption metrics.
    sensor: Box<dyn Sensor>,
}

impl RiemannExporter {
    /// Instantiates RiemannExporter and returns the instance.
    pub fn new(sensor: Box<dyn Sensor>) -> RiemannExporter {
        RiemannExporter { sensor }
    }
}

impl Exporter for RiemannExporter {
    /// Runs HTTP server and metrics exposure through the runner function.
    fn run(&mut self, parameters: ArgMatches) {
        let dispatch_duration: u64 = parameters
            .value_of("dispatch_duration")
            .unwrap()
            .parse()
            .expect("Wrong dispatch_duration value, should be a number of seconds");

        let hostname = String::from(
            hostname::get()
                .expect("Fail to get system hostname")
                .to_str()
                .unwrap(),
        );

        let mut rclient = Riemann::new(
            parameters.value_of("address").unwrap(),
            parameters.value_of("port").unwrap(),
        );

        info!("Starting Riemann exporter");
        println!("Press CTRL-C to stop scaphandre");
        println!("Measurement step is: {}s", dispatch_duration);

        loop {
            let mut topology = self.sensor.get_topology().unwrap();
            topology
                .proc_tracker
                .clean_terminated_process_records_vectors();
            topology.refresh();

            let mut host_energy_microjoules = String::from("NaN");
            let mut host_energy_timestamp_seconds = String::from("NaN");
            let records = topology.get_records_passive();
            if !records.is_empty() {
                let record = records.last().unwrap();
                host_energy_microjoules = record.value.clone();
                host_energy_timestamp_seconds = record.timestamp.as_secs().to_string();
            }

            rclient.send_metric(
                dispatch_duration as f32,
                &hostname,
                "self_version",
                "ok",
                vec!["scaphandre".to_string()],
                "Version number of scaphandre represented as a float.",
                get_scaphandre_version(),
            );

            if let Some(metric_value) = topology.get_process_cpu_consumption_percentage(
                procfs::process::Process::myself().unwrap().pid,
            ) {
                rclient.send_metric(
                    dispatch_duration as f32,
                    &hostname,
                    "self_cpu_usage_percent",
                    "ok",
                    vec!["scaphandre".to_string()],
                    "CPU % consumed by this scaphandre prometheus exporter.",
                    metric_value,
                );
            }

            if let Ok(metric_value) = procfs::process::Process::myself().unwrap().statm() {
                let value = metric_value.size * procfs::page_size().unwrap() as u64;
                rclient.send_metric(
                    dispatch_duration as f32,
                    &hostname,
                    "self_mem_total_program_size",
                    "ok",
                    vec!["scaphandre".to_string()],
                    "Total program size, measured in pages",
                    value,
                );

                let value = metric_value.resident * procfs::page_size().unwrap() as u64;
                rclient.send_metric(
                    dispatch_duration as f32,
                    &hostname,
                    "self_mem_resident_set_size",
                    "ok",
                    vec!["scaphandre".to_string()],
                    "Resident set size, measured in pages",
                    value,
                );

                let value = metric_value.size * procfs::page_size().unwrap() as u64;
                rclient.send_metric(
                    dispatch_duration as f32,
                    &hostname,
                    "self_mem_shared_resident_size",
                    "ok",
                    vec!["scaphandre".to_string()],
                    "Number of resident shared pages (i.e., backed by a file)",
                    value,
                );
            }

            let topo_stat_buffer_len = topology.stat_buffer.len();
            let topo_record_buffer_len = topology.record_buffer.len();
            let topo_procs_len = topology.proc_tracker.procs.len();
            rclient.send_metric(
                dispatch_duration as f32,
                &hostname,
                "self_topo_stats_nb",
                "ok",
                vec!["scaphandre".to_string()],
                "Number of CPUStat traces stored for the host",
                topo_stat_buffer_len,
            );

            rclient.send_metric(
                dispatch_duration as f32,
                &hostname,
                "self_topo_records_nb",
                "ok",
                vec!["scaphandre".to_string()],
                "Number of energy consumption Records stored for the host",
                topo_record_buffer_len,
            );

            rclient.send_metric(
                dispatch_duration as f32,
                &hostname,
                "self_topo_procs_nb",
                "ok",
                vec!["scaphandre".to_string()],
                "Number of processes monitored for the host",
                topo_procs_len,
            );

            rclient.send_metric(
                dispatch_duration as f32,
                &hostname,
                "self_topo_procs_nb",
                "ok",
                vec!["scaphandre".to_string()],
                "Number of processes monitored for the host",
                topo_procs_len,
            );

            for s in &topology.sockets {
                rclient.send_metric(
                    dispatch_duration as f32,
                    &hostname,
                    "self_socket_stats_nb",
                    "ok",
                    vec!["scaphandre".to_string()],
                    "Number of CPUStat traces stored for each socket",
                    s.stat_buffer.len(),
                );

                rclient.send_metric(
                    dispatch_duration as f32,
                    &hostname,
                    "self_socket_records_nb",
                    "ok",
                    vec!["scaphandre".to_string()],
                    "Number of energy consumption Records stored for each socket",
                    s.stat_buffer.len(),
                );

                for d in &s.domains {
                    rclient.send_metric(
                        dispatch_duration as f32,
                        &hostname,
                        "self_domain_records_nb",
                        "ok",
                        vec!["scaphandre".to_string()],
                        "Number of energy consumption Records stored for a Domain",
                        d.record_buffer.len(),
                    );
                }
            }

            thread::sleep(Duration::new(dispatch_duration, 0));
        }
    }

    /// Returns options understood by the exporter.
    fn get_options() -> HashMap<String, ExporterOption> {
        let mut options = HashMap::new();

        options.insert(
            String::from("address"),
            ExporterOption {
                default_value: String::from(DEFAULT_IP_ADDRESS),
                help: String::from("Riemann ipv6 or ipv4 address"),
                long: String::from("address"),
                short: String::from("a"),
                required: false,
                takes_value: true,
            },
        );
        options.insert(
            String::from("port"),
            ExporterOption {
                default_value: String::from(DEFAULT_PORT),
                help: String::from("Riemann TCP port number"),
                long: String::from("port"),
                short: String::from("p"),
                required: false,
                takes_value: true,
            },
        );
        options.insert(
            String::from("dispatch_duration"),
            ExporterOption {
                default_value: String::from("5"),
                help: String::from("Duration between metrics dispatch"),
                long: String::from("dispatch"),
                short: String::from("d"),
                required: false,
                takes_value: true,
            },
        );

        options
    }
}

fn get_scaphandre_version() -> String {
    let mut version_parts = crate_version!().split('.');
    let major_version = version_parts.next().unwrap();
    let patch_version = version_parts.next().unwrap();
    let minor_version = version_parts.next().unwrap();
    format!("{}.{}{}", major_version, patch_version, minor_version)
}
