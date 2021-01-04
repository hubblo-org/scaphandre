use crate::current_system_time_since_epoch;
use crate::exporters::*;
use crate::sensors::{Record, RecordGenerator, Sensor, Topology};
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use clap::crate_version;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::Duration;

/// Default ipv4/ipv6 address to expose the service is any
const DEFAULT_IP_ADDRESS: &str = "::";

/// Exporter that exposes metrics to an HTTP endpoint
/// matching the Riemann.io metrics format.
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
        println!("OK !");
    }
    /// Returns options understood by the exporter.
    fn get_options() -> HashMap<String, ExporterOption> {
        let mut options = HashMap::new();

        options.insert(
            String::from("address"),
            ExporterOption {
                default_value: String::from(DEFAULT_IP_ADDRESS),
                help: String::from("ipv6 or ipv4 address to expose the service to"),
                long: String::from("address"),
                short: String::from("a"),
                required: false,
                takes_value: true,
            },
        );
        options.insert(
            String::from("port"),
            ExporterOption {
                default_value: String::from("8080"),
                help: String::from("TCP port number to expose the service"),
                long: String::from("port"),
                short: String::from("p"),
                required: false,
                takes_value: true,
            },
        );
        options.insert(
            String::from("suffix"),
            ExporterOption {
                default_value: String::from("metrics"),
                help: String::from("url suffix to access metrics"),
                long: String::from("suffix"),
                short: String::from("s"),
                required: false,
                takes_value: true,
            },
        );

        options
    }
}
