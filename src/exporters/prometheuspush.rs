//! # PrometheusPushExporter
//!
//! `PrometheusPushExporter` implementation, push/send metrics to
//! a [Prometheus](https://prometheus.io/) pushgateway.
//! 
use clap::builder::TypedValueParser;
use isahc::{prelude::*, Request};
use std::time::Duration;
use crate::exporters::{Exporter};
use crate::sensors::{Sensor};
use clap::{ArgMatches, Arg};
use chrono::Utc;
use std::thread;

pub struct PrometheusPushExporter {
    sensor: Box<dyn Sensor>,
}

impl PrometheusPushExporter {
    pub fn new(sensor: Box<dyn Sensor>) -> PrometheusPushExporter {
        PrometheusPushExporter { sensor }
    }
}

impl Exporter for PrometheusPushExporter {
    fn run(&mut self, parameters: ArgMatches) {
        info!(
            "{}: Starting Prometheus Push exporter",
            Utc::now().format("%Y-%m-%dT%H:%M:%S")
        );

        let step: String = *parameters.get_one("step").unwrap();
        let host: String = *parameters.get_one("host").unwrap();
        let scheme: String = *parameters.get_one("scheme").unwrap();
        let port: String = *parameters.get_one("port").unwrap();
        let route: String = *parameters.get_one("route").unwrap();
        let uri = format!("{scheme}://{host}:{port}/{route}");

        loop {
            let body = "# HELP mymetric this is my metric\n# TYPE mymetric gauge\nmymetric 50";
            if let Ok(request) = Request::post(uri.clone())
                .header("Content-Type", "text/plain")
                .timeout(Duration::from_secs(5))
                .body(body) {
                    match request.send() {
                        Ok(response) => {
                            warn!("Got {:?}", response);
                        }
                        Err(err) => {
                            warn!("Got error : {:?}", err)
                        }
                    }
                }

            thread::sleep(Duration::new(step.parse::<u64>().unwrap(), 0));
        }
    }
    /// Returns options understood by the exporter.
    fn get_options() -> Vec<clap::Arg> {
        let mut options = Vec::new();
        let arg = Arg::new("host")
            .default_value("localhost")
            .help("PushGateway's host FQDN or IP address.")
            .long("host")
            .short('H')
            .required(false) // send to localhost if none
            .action(clap::ArgAction::Set);
        options.push(arg);
        let arg = Arg::new("port")
            .default_value("9091")
            .help("PushGateway's TCP port number.")
            .long("port")
            .short('p')
            .required(false) // send to localhost if none
            .action(clap::ArgAction::Set);
        options.push(arg);
        let arg = Arg::new("scheme")
            .default_value("https")
            .help("http or https.")
            .long("scheme")
            .short('s')
            .required(false) // send to localhost if none
            .action(clap::ArgAction::Set);
        options.push(arg);
        let arg = Arg::new("step")
            .default_value("20")
            .help("Time between two push, in seconds.")
            .long("step")
            .short('S')
            .required(false) // send to localhost if none
            .action(clap::ArgAction::Set);
        options.push(arg);

        options
    }
}