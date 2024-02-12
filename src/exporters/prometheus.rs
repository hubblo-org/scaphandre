//! # PrometheusExporter
//!
//! The Prometheus Exporter expose metrics to a [Prometheus](https://prometheus.io/) server.
//! This is achieved by exposing an HTTP endpoint, which the Prometheus will
//! [scrape](https://prometheus.io/docs/prometheus/latest/getting_started).

use super::utils;
use crate::exporters::{Exporter, MetricGenerator, MetricValueType};
use crate::sensors::utils::current_system_time_since_epoch;
use crate::sensors::{Sensor, Topology};
use chrono::Utc;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use std::{
    collections::HashMap,
    fmt::Write,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
    time::Duration,
};

/// Default ipv4/ipv6 address to expose the service is any
const DEFAULT_IP_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));

/// Exporter that exposes metrics to an HTTP endpoint
/// matching the Prometheus.io metrics format.
pub struct PrometheusExporter {
    topo: Topology,
    hostname: String,
    args: ExporterArgs,
}

/// Hold the arguments for a PrometheusExporter.
#[derive(clap::Args, Debug)]
pub struct ExporterArgs {
    /// IP address (v4 or v6) of the metrics endpoint for Prometheus
    #[arg(short, long, default_value_t = DEFAULT_IP_ADDRESS)]
    pub address: IpAddr,

    /// TCP port of the metrics endpoint for Prometheus
    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,

    #[arg(short, long, default_value_t = String::from("metrics"))]
    pub suffix: String,

    /// Apply labels to metrics of processes that look like a Qemu/KVM virtual machine
    #[arg(long)]
    pub qemu: bool,

    /// Apply labels to metrics of processes running as containers
    #[arg(long)]
    pub containers: bool,
}

impl PrometheusExporter {
    /// Instantiates PrometheusExporter and returns the instance.
    pub fn new(sensor: &dyn Sensor, args: ExporterArgs) -> PrometheusExporter {
        // Prepare the retrieval of the measurements, catch most of the errors early
        let topo = sensor
            .get_topology()
            .expect("sensor topology should be available");
        let hostname = utils::get_hostname();
        PrometheusExporter {
            topo,
            hostname,
            args,
        }
    }
}

impl Exporter for PrometheusExporter {
    /// Starts an HTTP server to expose the metrics in Prometheus format.
    fn run(&mut self) {
        info!(
            "{}: Starting Prometheus exporter",
            Utc::now().format("%Y-%m-%dT%H:%M:%S")
        );
        println!("Press CTRL-C to stop scaphandre");
        let socket_addr = SocketAddr::new(self.args.address, self.args.port);
        let metric_generator = MetricGenerator::new(
            self.topo.clone(), // improvement possible here: avoid cloning by adding a lifetime param to MetricGenerator
            self.hostname.clone(),
            self.args.qemu,
            self.args.containers,
        );
        run_server(socket_addr, metric_generator, &self.args.suffix);
    }

    fn kind(&self) -> &str {
        "prometheus"
    }
}

/// Contains a mutex holding a MetricGenerator.
/// Used to pass the topology data from one http worker to another.
struct PowerMetrics {
    last_request: Mutex<Duration>,
    metric_generator: Mutex<MetricGenerator>,
}

#[tokio::main]
async fn run_server(
    socket_addr: SocketAddr,
    metric_generator: MetricGenerator,
    endpoint_suffix: &str,
) {
    let power_metrics = PowerMetrics {
        last_request: Mutex::new(Duration::new(0, 0)),
        metric_generator: Mutex::new(metric_generator),
    };
    let context = Arc::new(power_metrics);
    let make_svc = make_service_fn(move |_| {
        let ctx = context.clone();
        let sfx = endpoint_suffix.to_string();
        async {
            Ok::<_, Infallible>(service_fn(move |req| {
                show_metrics(req, ctx.clone(), sfx.clone())
            }))
        }
    });
    let server = Server::bind(&socket_addr);
    let res = server.serve(make_svc);
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let graceful = res.with_graceful_shutdown(async {
        rx.await.ok();
    });

    if let Err(e) = graceful.await {
        error!("server error: {}", e);
    }
    let _ = tx.send(());
}

/// Adds lines related to a metric in the body (String) of response.
fn push_metric(
    mut body: String,
    help: String,
    metric_type: String,
    metric_name: String,
    metric_line: String,
    add_help: bool,
) -> String {
    if add_help {
        let _ = write!(body, "# HELP {metric_name} {help}");
        let _ = write!(body, "\n# TYPE {metric_name} {metric_type}\n");
    }
    body.push_str(&metric_line);
    body
}

/// Handles requests and returns data formated for Prometheus.
async fn show_metrics(
    req: Request<Body>,
    context: Arc<PowerMetrics>,
    suffix: String,
) -> Result<Response<Body>, Infallible> {
    trace!("{}", req.uri());
    let mut body = String::new();
    if req.uri().path() == format!("/{}", &suffix) {
        let now = current_system_time_since_epoch();
        match context.last_request.lock() {
            Ok(mut last_request) => {
                match context.metric_generator.lock() {
                    Ok(mut metric_generator) => {
                        if now - (*last_request) > Duration::from_secs(2) {
                            {
                                info!(
                                    "{}: Refresh topology",
                                    Utc::now().format("%Y-%m-%dT%H:%M:%S")
                                );
                                metric_generator
                                    .topology
                                    .proc_tracker
                                    .clean_terminated_process_records_vectors();
                                metric_generator.topology.refresh();
                            }
                        }
                        *last_request = now;

                        info!("{}: Refresh data", Utc::now().format("%Y-%m-%dT%H:%M:%S"));

                        metric_generator.gen_all_metrics();

                        let mut metrics_pushed: Vec<String> = vec![];

                        // Send all data
                        for msg in metric_generator.pop_metrics() {
                            let mut attributes: Option<&HashMap<String, String>> = None;
                            if !msg.attributes.is_empty() {
                                attributes = Some(&msg.attributes);
                            }

                            let value = match msg.metric_value {
                                // MetricValueType::IntSigned(value) => event.set_metric_sint64(value),
                                // MetricValueType::Float(value) => event.set_metric_f(value),
                                //MetricValueType::FloatDouble(value) => value.to_string(),
                                MetricValueType::IntUnsigned(value) => value.to_string(),
                                MetricValueType::Text(ref value) => value.to_string(),
                            };

                            let mut should_i_add_help = true;

                            if metrics_pushed.contains(&msg.name) {
                                should_i_add_help = false;
                            } else {
                                metrics_pushed.insert(0, msg.name.clone());
                            }

                            body = push_metric(
                                body,
                                msg.description.clone(),
                                msg.metric_type.clone(),
                                msg.name.clone(),
                                utils::format_prometheus_metric(&msg.name, &value, attributes),
                                should_i_add_help,
                            );
                        }
                    }
                    Err(e) => {
                        error!("Error while locking metric_generator: {e:?}");
                        error!("Error while locking metric_generator: {}", e.to_string());
                    }
                }
            }
            Err(e) => {
                error!("Error in show_metrics : {e:?}");
                error!("Error details : {}", e.to_string());
            }
        }
    } else {
        let _ = write!(body, "<a href=\"https://github.com/hubblo-org/scaphandre/\">Scaphandre's</a> prometheus exporter here. Metrics available on <a href=\"/{suffix}\">/{suffix}</a>");
    }
    Ok(Response::new(body.into()))
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
