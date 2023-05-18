//! # PrometheusPushExporter
//!
//! `PrometheusPushExporter` implementation, push/send metrics to
//! a [Prometheus](https://prometheus.io/) pushgateway.
//!
use super::utils::{format_prometheus_metric, get_hostname};
use crate::exporters::{Exporter, MetricGenerator};
use crate::sensors::{Sensor, Topology};
use chrono::Utc;
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

    #[arg(short, long, default_value_t = 5)]
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
            "{}://{}:{}/{}/job/{}",
            self.args.scheme, self.args.host, self.args.port, self.args.suffix, self.args.job
        );

        let mut metric_generator = MetricGenerator::new(
            self.topo.clone(),
            self.hostname.clone(),
            self.args.qemu,
            self.args.containers,
        );
        // add job and per metric suffix ?

        loop {
            metric_generator.topology.refresh();
            metric_generator.gen_all_metrics();
            let mut body = String::from(
                "# HELP mymetric this is my metric\n# TYPE mymetric gauge\nmymetric 50\n",
            );
            let mut metrics_pushed: Vec<String> = vec![];
            //let mut counter = 0;
            for m in metric_generator.pop_metrics() {
                let mut should_i_add_help = true;

                if metrics_pushed.contains(&m.name) {
                    should_i_add_help = false;
                } else {
                    metrics_pushed.insert(0, m.name.clone());
                }

                if should_i_add_help {
                    let _ = write!(body, "# HELP {} {}", m.name, m.description);
                    //warn!(
                    //    "line {} : {}",
                    //    counter,
                    //    format!("# HELP {} {}", m.name, m.description)
                    //);
                    //counter = counter + 1;
                    let _ = write!(body, "\n# TYPE {} {}\n", m.name, m.metric_type);
                    //warn!(
                    //    "line {} : {}",
                    //    counter,
                    //    format!("\n# TYPE {} {}\n", m.name, m.metric_type)
                    //);
                    //counter = counter + 1;
                }
                let mut attributes = None;
                if !m.attributes.is_empty() {
                    attributes = Some(&m.attributes);
                }
                //warn!(
                //    "line {} : {}",
                //    counter,
                //    format_prometheus_metric(&m.name, &m.metric_value.to_string(), attributes)
                //);
                //counter = counter + 1;
                let _ = write!(
                    body,
                    "{}",
                    format_prometheus_metric(&m.name, &m.metric_value.to_string(), attributes)
                );
            }
            //warn!("body: {}", body);
            // TODO: fix tcp broken pipe on push gateway side
            if let Ok(request) = Request::post(uri.clone())
                .header("Content-Type", "text/plain")
                .timeout(Duration::from_secs(5))
                .body(body)
            {
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
