//! # PrometheusExporter
//!
//! `PrometheusExporter` implementation, expose metrics to
//! a [Prometheus](https://prometheus.io/) server.
use super::utils::get_hostname;
use crate::current_system_time_since_epoch;
use crate::exporters::{Exporter, MetricGenerator, MetricValueType};
use crate::sensors::{Sensor, Topology};
use chrono::Utc;
use clap::{Arg, ArgMatches};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::Duration,
};

/// Default ipv4/ipv6 address to expose the service is any
const DEFAULT_IP_ADDRESS: &str = "::";

/// Exporter that exposes metrics to an HTTP endpoint
/// matching the Prometheus.io metrics format.
pub struct PrometheusExporter {
    /// Sensor instance that is used to generate the Topology and
    /// thus get power consumption metrics.
    sensor: Box<dyn Sensor>,
}

impl PrometheusExporter {
    /// Instantiates PrometheusExporter and returns the instance.
    pub fn new(sensor: Box<dyn Sensor>) -> PrometheusExporter {
        PrometheusExporter { sensor }
    }
}

impl Exporter for PrometheusExporter {
    /// Entry point ot the PrometheusExporter.
    ///
    /// Runs HTTP server and metrics exposure through the runner function.
    fn run(&mut self, parameters: ArgMatches) {
        info!(
            "{}: Starting Prometheus exporter",
            Utc::now().format("%Y-%m-%dT%H:%M:%S")
        );
        println!("Press CTRL-C to stop scaphandre");

        runner(
            (*self.sensor.get_topology()).unwrap(),
            parameters.value_of("address").unwrap().to_string(),
            parameters.value_of("port").unwrap().to_string(),
            parameters.value_of("suffix").unwrap().to_string(),
            parameters.is_present("qemu"),
            parameters.is_present("containers"),
            get_hostname(),
        );
    }
    /// Returns options understood by the exporter.
    fn get_options() -> Vec<clap::Arg<'static, 'static>> {
        let mut options = Vec::new();
        let arg = Arg::with_name("address")
            .default_value(DEFAULT_IP_ADDRESS)
            .help("ipv6 or ipv4 address to expose the service to")
            .long("address")
            .short("a")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("port")
            .default_value("8080")
            .help("TCP port number to expose the service")
            .long("port")
            .short("p")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("suffix")
            .default_value("metrics")
            .help("url suffix to access metrics")
            .long("suffix")
            .short("s")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("qemu")
            .help("Apply labels to metrics of processes looking like a Qemu/KVM virtual machine")
            .long("qemu")
            .short("q")
            .required(false)
            .takes_value(false);
        options.push(arg);

        let arg = Arg::with_name("containers")
            .help("Monitor and apply labels for processes running as containers")
            .long("containers")
            .required(false)
            .takes_value(false);
        options.push(arg);

        let arg = Arg::with_name("kubernetes_host")
            .help("FQDN of the kubernetes API server")
            .long("kubernetes-host")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("kubernetes_scheme")
            .help("Protocol used to access kubernetes API server")
            .long("kubernetes-scheme")
            .default_value("http")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("kubernetes_port")
            .help("Kubernetes API server port number")
            .long("kubernetes-port")
            .default_value("6443")
            .required(false)
            .takes_value(true);
        options.push(arg);

        options
    }
}

/// Contains a mutex holding a Topology object.
/// Used to pass the topology data from one http worker to another.
struct PowerMetrics {
    last_request: Mutex<Duration>,
    metric_generator: Mutex<MetricGenerator>,
}

#[tokio::main]
async fn runner(
    topology: Topology,
    address: String,
    port: String,
    suffix: String,
    qemu: bool,
    watch_containers: bool,
    hostname: String,
) {
    if let Ok(addr) = address.parse::<IpAddr>() {
        if let Ok(port) = port.parse::<u16>() {
            let socket_addr = SocketAddr::new(addr, port);

            let power_metrics = PowerMetrics {
                last_request: Mutex::new(Duration::new(0, 0)),
                metric_generator: Mutex::new(MetricGenerator::new(
                    topology,
                    hostname.clone(),
                    qemu,
                    watch_containers,
                )),
            };
            let context = Arc::new(power_metrics);
            let make_svc = make_service_fn(move |_| {
                let ctx = context.clone();
                let sfx = suffix.clone();
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
        } else {
            panic!("{} is not a valid TCP port number", port);
        }
    } else {
        panic!("{} is not a valid ip address", address);
    }
}

/// Returns a well formatted Prometheus metric string.
fn format_metric(key: &str, value: &str, labels: Option<&HashMap<String, String>>) -> String {
    let mut result = key.to_string();
    if let Some(labels) = labels {
        result.push('{');
        for (k, v) in labels.iter() {
            result.push_str(&format!("{}=\"{}\",", k, v.replace("\"", "_")));
        }
        result.remove(result.len() - 1);
        result.push('}');
    }
    result.push_str(&format!(" {}\n", value));
    result
}

/// Adds lines related to a metric in the body (String) of response.
fn push_metric(
    mut body: String,
    help: String,
    metric_type: String,
    metric_name: String,
    metric_line: String,
) -> String {
    body.push_str(&format!("# HELP {} {}", metric_name, help));
    body.push_str(&format!("\n# TYPE {} {}\n", metric_name, metric_type));
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
        trace!("in metrics !");
        let now = current_system_time_since_epoch();
        let mut last_request = context.last_request.lock().unwrap();
        let mut metric_generator = context.metric_generator.lock().unwrap();
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

        // Send all data
        for msg in metric_generator.pop_metrics() {
            let mut attributes: Option<&HashMap<String, String>> = None;
            if !msg.attributes.is_empty() {
                attributes = Some(&msg.attributes);
            }

            let value = match msg.metric_value {
                // MetricValueType::IntSigned(value) => event.set_metric_sint64(value),
                // MetricValueType::Float(value) => event.set_metric_f(value),
                MetricValueType::FloatDouble(value) => value.to_string(),
                MetricValueType::IntUnsigned(value) => value.to_string(),
                MetricValueType::Text(ref value) => value.to_string(),
            };
            body = push_metric(
                body,
                msg.description.clone(),
                msg.metric_type.clone(),
                msg.name.clone(),
                format_metric(&msg.name, &value, attributes),
            );
        }
    } else {
        body.push_str(&format!("<a href=\"https://github.com/hubblo-org/scaphandre/\">Scaphandre's</a> prometheus exporter here. Metrics available on <a href=\"/{}\">/{}</a>", suffix, suffix));
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
