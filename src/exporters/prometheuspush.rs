//! # PrometheusPushExporter
//!
//! `PrometheusPushExporter` implementation, push/send metrics to
//! a [Prometheus](https://prometheus.io/) pushgateway.
//! 
use isahc::{prelude::*, Request};
use std::time::Duration;
use crate::exporters::{Exporter};
use crate::sensors::{Sensor, Topology};
use chrono::Utc;
use std::thread;
use super::utils::get_hostname;

pub struct PrometheusPushExporter {
    topo: Topology,
    hostname: String,
    args: ExporterArgs
}

/// Hold the arguments for a PrometheusExporter.
#[derive(clap::Args, Debug)]
pub struct ExporterArgs {
    /// IP address (v4 or v6) of the metrics endpoint for Prometheus
    #[arg(short = 'H', long = "host", default_value_t = String::from("localhost"))]
    pub host: String,

    /// TCP port of the metrics endpoint for Prometheus
    #[arg(short, long, default_value_t = 9091)]
    pub port: u16,

    #[arg(long, default_value_t = String::from("metrics"))]
    pub suffix: String,

    #[arg(short = 'S', long, default_value_t = String::from("http"))]
    pub scheme: String,

    #[arg(short, long, default_value_t = 5)]
    pub step: u64,

    /// Apply labels to metrics of processes that look like a Qemu/KVM virtual machine
    #[arg(long)]
    pub qemu: bool,

    /// Apply labels to metrics of processes running as containers
    #[arg(long)]
    pub containers: bool,
}

impl PrometheusPushExporter {
    pub fn new(sensor: &dyn Sensor, args: ExporterArgs) -> PrometheusPushExporter {
        let topo = sensor
            .get_topology()
            .expect("sensor topology should be available");
        let hostname = get_hostname();
        PrometheusPushExporter { topo, hostname, args }
    }
}

impl Exporter for PrometheusPushExporter {
    fn run(&mut self) {
        info!(
            "{}: Starting Prometheus Push exporter",
            Utc::now().format("%Y-%m-%dT%H:%M:%S")
        );

        let uri = format!("{}://{}:{}/{}/job/test", self.args.scheme, self.args.host, self.args.port, self.args.suffix);
        // add job and per metric suffix ? 

        loop {
            let body = "# HELP mymetric this is my metric\n# TYPE mymetric gauge\nmymetric 50\n";
            if let Ok(request) = Request::post(uri.clone())
                .header("Content-Type", "text/plain")
                .timeout(Duration::from_secs(5))
                .body(body) {
                    match request.send() {
                        Ok(mut response) => {
                            warn!("Got {:?}", response);
                            warn!("Response Text {:?}", response.text());
                        }
                        Err(err) => {
                            warn!("Got error : {:?}", err)
                        }
                    }
                }

            thread::sleep(Duration::new(self.args.step, 0));
        }
    }

    fn kind(&self) -> &str {
        "prometheuspush"
    }
}