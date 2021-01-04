use crate::exporters::*;
use crate::sensors::{RecordGenerator, Sensor};
use clap::crate_version;
use riemann_client::proto::Event;
use riemann_client::Client;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

/// Riemann server default ipv4/ipv6 address
const DEFAULT_IP_ADDRESS: &str = "localhost";

/// Riemann server default port
const DEFAULT_PORT: &str = "5555";

trait Metric {
    fn add_metric(self, event: &mut Event);
}

impl Metric for i64 {
    fn add_metric(self, event: &mut Event) {
        event.set_metric_sint64(self);
    }
}

impl Metric for f32 {
    fn add_metric(self, event: &mut Event) {
        event.set_metric_f(self);
    }
}

impl Metric for f64 {
    fn add_metric(self, event: &mut Event) {
        event.set_metric_d(self);
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

    fn send_metric<T: Metric>(
        &mut self,
        hostname: String,
        service: String,
        state: String,
        metric: T,
    ) {
        let mut event = Event::new();
        event.set_host(hostname);
        event.set_service(service);
        event.set_state(state);
        metric.add_metric(&mut event);
        self.client.event(event);
        //self.s
        //self.event(event);
        //metric.add_metric(event);
        // rclient
        //     .event({
        //         let mut event = Event::new();
        //         event.set_service("rust-riemann_client".to_string());
        //         event.set_host(hostname.clone());
        //         event.set_state("ok".to_string());
        //         event.set_metric_f(scaphandre_version.parse::<f32>().unwrap());
        //         event
        //     })
        //     .unwrap();
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
            let metric_name = "self_version";
            let mut version_parts = crate_version!().split('.');
            let major_version = version_parts.next().unwrap();
            let patch_version = version_parts.next().unwrap();
            let minor_version = version_parts.next().unwrap();
            let scaphandre_version =
                format!("{}.{}{}", major_version, patch_version, minor_version);
            println!("version:{}", scaphandre_version);

            let metric_name = "self_cpu_usage_percent";
            if let Some(metric_value) = topology.get_process_cpu_consumption_percentage(
                procfs::process::Process::myself().unwrap().pid,
            ) {
                println!("{}={}", metric_name, &metric_value.to_string());
            }

            let metric_gathering = procfs::process::Process::myself().unwrap().statm();
            if let Ok(metric_value) = metric_gathering {
                let metric_name = "self_mem_total_program_size";
                let value = metric_value.size * procfs::page_size().unwrap() as u64;
                println!("{}={}", metric_name, value);

                let metric_name = "self_mem_resident_set_size";
                let value = metric_value.resident * procfs::page_size().unwrap() as u64;
                println!("{}={}", metric_name, value);

                let metric_name = "self_mem_shared_resident_size";
                let value = metric_value.size * procfs::page_size().unwrap() as u64;
                println!("{}={}", metric_name, value);
            }

            // rclient
            //     .event({
            //         let mut event = Event::new();
            //         event.set_service("rust-riemann_client".to_string());
            //         event.set_host(hostname.clone());
            //         event.set_state("ok".to_string());
            //         event.set_metric_f(scaphandre_version.parse::<f32>().unwrap());
            //         event
            //     })
            //     .unwrap();
            rclient.send_metric(hostname.clone(), "nene".to_string(), "ok".to_string(), 2.0);
            rclient.send_metric(hostname.clone(), "nene2".to_string(), "ok".to_string(), 3);

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
