use crate::exporters::*;
use crate::sensors::{RecordGenerator, Sensor};
use riemann_client::proto::Attribute;
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
trait Rmetric {
    fn set_metric(self, event: &mut Event);
}

impl Rmetric for usize {
    fn set_metric(self, event: &mut Event) {
        event.set_metric_sint64(self as i64);
    }
}

impl Rmetric for u64 {
    fn set_metric(self, event: &mut Event) {
        event.set_metric_sint64(self as i64);
    }
}

impl Rmetric for u32 {
    fn set_metric(self, event: &mut Event) {
        event.set_metric_sint64(self as i64);
    }
}

impl Rmetric for isize {
    fn set_metric(self, event: &mut Event) {
        event.set_metric_sint64(self as i64);
    }
}

impl Rmetric for i64 {
    fn set_metric(self, event: &mut Event) {
        event.set_metric_sint64(self);
    }
}

impl Rmetric for i32 {
    fn set_metric(self, event: &mut Event) {
        event.set_metric_sint64(self as i64);
    }
}

impl Rmetric for f32 {
    fn set_metric(self, event: &mut Event) {
        event.set_metric_f(self);
    }
}

impl Rmetric for f64 {
    fn set_metric(self, event: &mut Event) {
        event.set_metric_d(self);
    }
}

impl Rmetric for &str {
    fn set_metric(self, event: &mut Event) {
        let metric = self.replace(",", ".").replace("\n", "");
        if metric.contains('.') {
            event.set_metric_d(metric.parse::<f64>().expect("Cannot parse metric"));
        } else {
            event.set_metric_sint64(metric.parse::<i64>().expect("Cannot parse metric"));
        }
    }
}

impl Rmetric for String {
    fn set_metric(self, event: &mut Event) {
        let metric = self.replace(",", ".").replace("\n", "");
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
    fn send_metric<T: Rmetric>(
        &mut self,
        ttl: f32,
        hostname: &str,
        service: &str,
        state: &str,
        tags: Vec<String>,
        attributes: Vec<Attribute>,
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
        event.set_tags(protobuf::RepeatedField::from_vec(tags));
        if !attributes.is_empty() {
            event.set_attributes(protobuf::RepeatedField::from_vec(attributes));
        }
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
    // fn manage_metric<T>(&self, client: T, data: &Vec<Metric>) {
    //     unimplemented!()
    // }
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

        let mut data: Vec<Metric> = vec![];

        info!("Starting Riemann exporter");
        println!("Press CTRL-C to stop scaphandre");
        println!("Measurement step is: {}s", dispatch_duration);

        let mut topology = self.sensor.get_topology().unwrap();
        loop {
            debug!("Beginning of measure loop");
            topology
                .proc_tracker
                .clean_terminated_process_records_vectors();
            debug!("Refresh topology");
            topology.refresh();

            let records = topology.get_records_passive();

            self.get_self_metrics(&topology, &mut data, &hostname);
            println!("data: {:?}", data);

            // This should be the lastest part of the run method
            for msg in &data {
                let mut attributes: Vec<Attribute> = vec![];
                for (key, value) in &msg.attributes {
                    let mut attribute = Attribute::new();
                    attribute.set_key(key.clone());
                    attribute.set_value(value.clone());
                    attributes.push(attribute);
                }

                rclient.send_metric(
                    msg.ttl,
                    &msg.hostname,
                    &msg.name,
                    &msg.state,
                    msg.tags.clone(),
                    attributes,
                    &msg.description,
                    msg.metric,
                );
            }

            for socket in &topology.sockets {
                let mut attribute = Attribute::new();
                attribute.set_key("socket_id".to_string());
                attribute.set_value(socket.id.to_string());
                rclient.send_metric(
                    60.0,
                    &hostname,
                    "scaph_self_socket_stats_nb",
                    "ok",
                    vec!["scaphandre".to_string()],
                    vec![attribute.clone()],
                    "Number of CPUStat traces stored for each socket",
                    socket.stat_buffer.len(),
                );

                rclient.send_metric(
                    60.0,
                    &hostname,
                    "scaph_self_socket_records_nb",
                    "ok",
                    vec!["scaphandre".to_string()],
                    vec![attribute.clone()],
                    "Number of energy consumption Records stored for each socket",
                    socket.stat_buffer.len(),
                );

                for domain in &socket.domains {
                    let mut attribute = Attribute::new();
                    attribute.set_key("rapl_domain_name".to_string());
                    attribute.set_value(domain.name.to_string());
                    rclient.send_metric(
                        60.0,
                        &hostname,
                        "scaph_self_domain_records_nb",
                        "ok",
                        vec!["scaphandre".to_string()],
                        vec![attribute.clone()],
                        "Number of energy consumption Records stored for a Domain",
                        domain.record_buffer.len(),
                    );
                }
            }

            // metrics
            if !records.is_empty() {
                let record = records.last().unwrap();
                let host_energy_microjoules = record.value.clone();
                let host_energy_timestamp_seconds = record.timestamp.as_secs().to_string();

                rclient.send_metric(
                60.0,
                &hostname,
                "scaph_host_energy_microjoules",
                "ok",
                vec!["scaphandre".to_string()],
                vec![],
                "Energy measurement for the whole host, as extracted from the sensor, in microjoules.",
                host_energy_microjoules
            );

                rclient.send_metric(
                    60.0,
                    &hostname,
                    "scaph_host_energy_timestamp_seconds",
                    "ok",
                    vec!["scaphandre".to_string()],
                    vec![],
                    "Timestamp in seconds when host_energy_microjoules has been computed.",
                    host_energy_timestamp_seconds,
                );

                if let Some(power) = topology.get_records_diff_power_microwatts() {
                    rclient.send_metric(
                        60.0,
                        &hostname,
                        "scaph_host_power_microwatts",
                        "ok",
                        vec!["scaphandre".to_string()],
                        vec![],
                        "Power measurement on the whole host, in microwatts",
                        power.value,
                    );
                }
            }

            let sockets = topology.get_sockets_passive();
            for socket in sockets {
                let records = socket.get_records_passive();
                if !records.is_empty() {
                    let socket_energy_microjoules = &records.last().unwrap().value;

                    let mut attribute = Attribute::new();
                    attribute.set_key("socket_id".to_string());
                    attribute.set_value(socket.id.to_string());
                    rclient.send_metric(
                        60.0,
                        &hostname,
                        "scaph_socket_energy_microjoules",
                        "ok",
                        vec!["scaphandre".to_string()],
                        vec![attribute.clone()],
                        "Socket related energy measurement in microjoules.",
                        socket_energy_microjoules.as_ref(),
                    );

                    if let Some(power) = topology.get_records_diff_power_microwatts() {
                        let socket_power_microwatts = &power.value;

                        rclient.send_metric(
                            60.0,
                            &hostname,
                            "scaph_socket_power_microwatts",
                            "ok",
                            vec!["scaphandre".to_string()],
                            vec![attribute.clone()],
                            "Power measurement relative to a CPU socket, in microwatts",
                            socket_power_microwatts.as_ref(),
                        );
                    }
                }
            }

            if let Some(metric_value) = topology.read_nb_process_total_count() {
                rclient.send_metric(
                    60.0,
                    &hostname,
                    "scaph_forks_since_boot_total",
                    "ok",
                    vec!["scaphandre".to_string()],
                    vec![],
                    "Number of forks that have occured since boot (number of processes to have existed so far).",
                    metric_value,
                );
            }

            if let Some(metric_value) = topology.read_nb_process_running_current() {
                rclient.send_metric(
                    60.0,
                    &hostname,
                    "scaph_processes_running_current",
                    "ok",
                    vec!["scaphandre".to_string()],
                    vec![],
                    "Number of processes currently running.",
                    metric_value,
                );
            }

            if let Some(metric_value) = topology.read_nb_process_blocked_current() {
                rclient.send_metric(
                    60.0,
                    &hostname,
                    "scaph_processes_blocked_current",
                    "ok",
                    vec!["scaphandre".to_string()],
                    vec![],
                    "Number of processes currently blocked waiting for I/O.",
                    metric_value,
                );
            }

            if let Some(metric_value) = topology.read_nb_context_switches_total_count() {
                rclient.send_metric(
                    60.0,
                    &hostname,
                    "scaph_context_switches_total",
                    "ok",
                    vec!["scaphandre".to_string()],
                    vec![],
                    "Number of context switches since boot.",
                    metric_value,
                );
            }

            let processes_tracker = &topology.proc_tracker;

            for pid in processes_tracker.get_alive_pids() {
                let exe = processes_tracker.get_process_name(pid);
                let cmdline = processes_tracker.get_process_cmdline(pid);
                let mut attributes = vec![];

                let mut attribute = Attribute::new();
                attribute.set_key("pid".to_string());
                attribute.set_value(pid.to_string());
                attributes.push(attribute);

                let mut attribute = Attribute::new();
                attribute.set_key("exe".to_string());
                attribute.set_value(exe.clone());
                attributes.push(attribute);

                if let Some(cmdline_str) = cmdline {
                    let mut attribute = Attribute::new();
                    attribute.set_key("cmdline".to_string());
                    attribute.set_value(cmdline_str.replace("\"", "\\\""));
                    attributes.push(attribute);

                    if parameters.is_present("qemu") {
                        if let Some(vmname) = utils::filter_qemu_cmdline(&cmdline_str) {
                            let mut attribute = Attribute::new();
                            attribute.set_key("vmname".to_string());
                            attribute.set_value(vmname);
                            attributes.push(attribute);
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
                    rclient.send_metric(
                        60.0,
                        &hostname,
                        &metric_name,
                        "ok",
                        vec!["scaphandre".to_string()],
                        attributes,
                        "Power consumption due to the process, measured on at the topology level, in microwatts",
                        power.to_string(),
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
                default_value: Some(String::from(DEFAULT_IP_ADDRESS)),
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
                default_value: Some(String::from(DEFAULT_PORT)),
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
                default_value: Some(String::from("5")),
                help: String::from("Duration between metrics dispatch"),
                long: String::from("dispatch"),
                short: String::from("d"),
                required: false,
                takes_value: true,
            },
        );
        options.insert(
            String::from("qemu"),
            ExporterOption {
                default_value: None,
                help: String::from("Instruct that scaphandre is running on an hypervisor"),
                long: String::from("qemu"),
                short: String::from("q"),
                required: false,
                takes_value: false,
            },
        );

        options
    }
}
