//! # PrometheusPushExporter
//!
//! `PrometheusPushExporter` implementation, push/send metrics to
//! a [Prometheus](https://prometheus.io/) pushgateway.
//!

use super::utils::{format_prometheus_metric, get_hostname};
use crate::exporters::{Exporter, MetricGenerator};
use crate::sensors::{Sensor, Topology};
use chrono::Utc;
use isahc::config::SslOption;
use isahc::{prelude::*, Request};
use std::fmt::Write;
use std::thread;
use std::time::Duration;

pub struct PrometheusPushExporter {
    topo: Topology,
    hostname: String,
    args: ExporterArgs,
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

    #[arg(short, long, default_value_t = 30)]
    pub step: u64,

    /// Apply labels to metrics of processes that look like a Qemu/KVM virtual machine
    #[arg(long)]
    pub qemu: bool,

    /// Apply labels to metrics of processes running as containers
    #[arg(long)]
    pub containers: bool,

    /// Job name to apply as a label for pushed metrics
    #[arg(short, long, default_value_t = String::from("scaphandre"))]
    pub job: String,

    /// Don't verify remote TLS certificate (works with --scheme="https")
    #[arg(long)]
    pub no_tls_check: bool,
}

impl PrometheusPushExporter {
    pub fn new(sensor: &dyn Sensor, args: ExporterArgs) -> PrometheusPushExporter {
        let topo = sensor
            .get_topology()
            .expect("sensor topology should be available");
        let hostname = get_hostname();
        PrometheusPushExporter {
            topo,
            hostname,
            args,
        }
    }
}

impl Exporter for PrometheusPushExporter {
    fn run(&mut self) {
        info!(
            "{}: Starting Prometheus Push exporter",
            Utc::now().format("%Y-%m-%dT%H:%M:%S")
        );

        let uri = format!(
            "{}://{}:{}/{}/job/{}/instance/{}",
            self.args.scheme,
            self.args.host,
            self.args.port,
            self.args.suffix,
            self.args.job,
            self.hostname.clone()
        );

        let mut metric_generator = MetricGenerator::new(
            self.topo.clone(),
            self.hostname.clone(),
            self.args.qemu,
            self.args.containers,
        );

        loop {
            metric_generator.topology.refresh();
            metric_generator.gen_all_metrics();
            let mut body = String::from("");
            let mut metrics_pushed: Vec<String> = vec![];
            //let mut counter = 0;
            for mut m in metric_generator.pop_metrics() {
                let mut should_i_add_help = true;

                if metrics_pushed.contains(&m.name) {
                    should_i_add_help = false;
                } else {
                    metrics_pushed.insert(0, m.name.clone());
                }

                if should_i_add_help {
                    let _ = write!(body, "# HELP {} {}", m.name, m.description);
                    let _ = write!(body, "\n# TYPE {} {}\n", m.name, m.metric_type);
                }
                if !&m.attributes.contains_key("instance") {
                    m.attributes
                        .insert(String::from("instance"), m.hostname.clone());
                }
                if !&m.attributes.contains_key("hostname") {
                    m.attributes
                        .insert(String::from("hostname"), m.hostname.clone());
                }
                let attributes = Some(&m.attributes);

                let _ = write!(
                    body,
                    "{}",
                    format_prometheus_metric(&m.name, &m.metric_value.to_string(), attributes)
                );
            }

            let pre_request = Request::post(uri.clone())
                .timeout(Duration::from_secs(5))
                .header("Content-Type", "text/plain");
            let final_request = match self.args.no_tls_check {
                true => pre_request.ssl_options(
                    SslOption::DANGER_ACCEPT_INVALID_CERTS
                        | SslOption::DANGER_ACCEPT_REVOKED_CERTS
                        | SslOption::DANGER_ACCEPT_INVALID_HOSTS,
                ),
                false => pre_request,
            };
            if let Ok(request) = final_request.body(body) {
                match request.send() {
                    Ok(mut response) => {
                        debug!("Got {:?}", response);
                        debug!("Response Text {:?}", response.text());
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
