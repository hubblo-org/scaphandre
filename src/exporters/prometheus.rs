use crate::current_system_time_since_epoch;
use crate::exporters::*;
use crate::sensors::{Record, RecordGenerator, Sensor, Topology};
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::Duration;

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
    /// Runs HTTP server and metrics exposure through the runner function.
    fn run(&mut self, parameters: ArgMatches) {
        match runner(
            (*self.sensor.get_topology()).unwrap(),
            parameters.value_of("address").unwrap().to_string(),
            parameters.value_of("port").unwrap().to_string(),
            parameters.value_of("suffix").unwrap().to_string(),
        ) {
            Ok(()) => warn!("Prometheus exporter shut down gracefully."),
            Err(error) => panic!("Something failed in the prometheus exporter: {}", error),
        }
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

/// Contains a mutex holding a Topology object.
/// Used to pass the topology data from one http worker to another.
pub struct PowerMetrics {
    topology: Mutex<Topology>,
    last_request: Mutex<Duration>,
}

#[actix_web::main]
/// Main function running the HTTP server.
async fn runner(
    topology: Topology,
    address: String,
    port: String,
    suffix: String,
) -> std::io::Result<()> {
    if let Err(error) = address.parse::<IpAddr>() {
        panic!("{} is not a valid ip address: {}", address, error);
    }
    if let Err(error) = port.parse::<u64>() {
        panic!("Not a valid TCP port numer: {}", error);
    }
    HttpServer::new(move || {
        App::new()
            .data(PowerMetrics {
                topology: Mutex::new(topology.clone()),
                last_request: Mutex::new(Duration::new(0, 0)),
            })
            .service(web::resource(&suffix).route(web::get().to(show_metrics)))
            .default_service(web::route().to(landing_page))
    })
    .workers(1)
    .bind(format!("{}:{}", address, port))?
    .run()
    .await
}

/// Returns a well formatted Prometheus metric string.
fn format_metric(key: &str, value: &str, labels: Option<&HashMap<String, String>>) -> String {
    let prefix = "scaph";
    let mut result = format!("{}_{}", prefix, key);
    if let Some(labels) = labels {
        result.push('{');
        for (k, v) in labels.iter() {
            result.push_str(&format!("{}=\"{}\",", k, v));
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

/// Handles requests and returns data.
async fn show_metrics(data: web::Data<PowerMetrics>) -> impl Responder {
    let now = current_system_time_since_epoch();
    let mut last_request = data.last_request.lock().unwrap();

    if now - (*last_request) > Duration::from_secs(5) {
        {
            warn!("Updating topo !");
            let mut topology = data.topology.lock().unwrap();
            (*topology)
                .proc_tracker
                .clean_terminated_process_records_vectors();
            (*topology).refresh();
            let stat_buffer_len = (*topology).stat_buffer.len();
            //let stat_buffer_size = size_of_val(&(*topology).stat_buffer.get(0).unwrap()) *  stat_buffer_len;
            let record_buffer_len = (*topology).record_buffer.len();
            //let record_buffer_size: size_of_val(&(*topology).record_buffer.get(0).unwrap()) * record_buffer_len;
            let procs_len = (*topology).proc_tracker.procs.len();
            info!(
                "sizeof topo stats len: {} records len: {} procs len: {}",
                stat_buffer_len, //stat_buffer_size,
                record_buffer_len,// record_buffer_size,
                procs_len
            );
        }
    }

    *last_request = now;
    let topo = data.topology.lock().unwrap();
    let records = (*topo).get_records_passive();
    let mut body = String::from(""); // initialize empty body
    let mut host_energy_microjoules = String::from("NaN");
    let mut host_energy_timestamp_seconds = String::from("NaN");
    if !records.is_empty() {
        let record = records.last().unwrap();
        host_energy_microjoules = record.value.clone();
        host_energy_timestamp_seconds = record.timestamp.as_secs().to_string();
    }

    // self metrics

    let metric_name = "self_cpu_usage_percent";
    if let Some(metric_value) = (*topo)
        .get_process_cpu_consumption_percentage(procfs::process::Process::myself().unwrap().pid)
    {
        body = push_metric(
            body,
            String::from("CPU % consumed by this scaphandre prometheus exporter."),
            String::from("gauge"),
            String::from(metric_name),
            format_metric(metric_name, &metric_value.to_string(), None),
        );
    }

    let metric_gathering = procfs::process::Process::myself().unwrap().statm();
    if let Ok(metric_value) = metric_gathering {
        let metric_name = "self_mem_total_program_size";
        let value = metric_value.size * procfs::page_size().unwrap() as u64;
        body = push_metric(
            body,
            String::from("Total program size, measured in pages"),
            String::from("gauge"),
            String::from(metric_name),
            format_metric(metric_name, &value.to_string(), None),
        );
        let metric_name = "self_mem_resident_set_size";
        let value = metric_value.resident * procfs::page_size().unwrap() as u64;
        body = push_metric(
            body,
            String::from("Resident set size, measured in pages"),
            String::from("gauge"),
            String::from(metric_name),
            format_metric(metric_name, &value.to_string(), None),
        );
        let metric_name = "self_mem_shared_resident_size";
        let value = metric_value.size * procfs::page_size().unwrap() as u64;
        body = push_metric(
            body,
            String::from("Number of resident shared pages (i.e., backed by a file)"),
            String::from("gauge"),
            String::from(metric_name),
            format_metric(metric_name, &value.to_string(), None),
        );
    }

    // metrics

    let metric_name = "host_energy_microjoules";
    body = push_metric(
        body,
        String::from(
            "Energy measurement for the whole host, as extracted from the sensor, in microjoules.",
        ),
        String::from("counter"),
        String::from(metric_name),
        format_metric(metric_name, &host_energy_microjoules, None),
    );

    let metric_name = "host_energy_timestamp_seconds";
    body = push_metric(
        body,
        String::from("Timestamp in seconds when hose_energy_microjoules has been computed."),
        String::from("counter"),
        String::from(metric_name),
        format_metric(metric_name, &host_energy_timestamp_seconds, None),
    );

    let mut host_power_microwatts = "Nan";
    let host_power_record: Record;
    if let Some(power) = (*topo).get_records_diff_power_microwatts() {
        host_power_record = power;
        host_power_microwatts = &host_power_record.value;
    }

    let metric_name = "host_power_microwatts";
    body = push_metric(
        body,
        String::from("Power measurement on the whole host, in microwatts"),
        String::from("gauge"),
        String::from(metric_name),
        format_metric(metric_name, host_power_microwatts, None),
    );

    let sockets = (*topo).get_sockets_passive();
    for s in sockets {
        let records = s.get_records_passive();
        let mut socket_energy_microjoules = "NaN";
        if !records.is_empty() {
            socket_energy_microjoules = &records.last().unwrap().value;
        }
        let mut labels = HashMap::new();
        labels.insert(String::from("socket_id"), s.id.to_string());

        let metric_name = "socket_energy_microjoules";
        body = push_metric(
            body,
            String::from("Socket related energy measurement in mirojoules."),
            String::from("counter"),
            String::from(metric_name),
            format_metric(metric_name, socket_energy_microjoules, Some(&labels)),
        );
        let mut socket_power_microwatts = "NaN";
        let socket_power_record: Record;
        if let Some(power) = (*topo).get_records_diff_power_microwatts() {
            socket_power_record = power;
            socket_power_microwatts = &socket_power_record.value;
        }

        let metric_name = "socket_power_microwatts";
        body = push_metric(
            body,
            String::from("Power measurement relative to a CPU socket, in microwatts"),
            String::from("gauge"),
            String::from(metric_name),
            format_metric(metric_name, socket_power_microwatts, Some(&labels)),
        );
    }

    let metric_name = "forks_since_boot_total";
    let mut metric_value_string = String::from("NaN");
    if let Some(metric_value) = &(*topo).read_nb_process_total_count() {
        metric_value_string = metric_value.to_string();
    }
    body = push_metric(
        body, String::from(
            "Number of forks that have occured since boot (number of processes to have existed so far)."
        ),
        String::from("counter"),
        String::from(metric_name),
        format_metric(
            metric_name, &metric_value_string, None
        )
    );

    let metric_name = "processes_running_current";
    let mut metric_value_string = String::from("NaN");
    if let Some(metric_value) = &(*topo).read_nb_process_running_current() {
        metric_value_string = metric_value.to_string();
    }
    body = push_metric(
        body,
        String::from("Number of processes currently running."),
        String::from("gauge"),
        String::from(metric_name),
        format_metric(metric_name, &metric_value_string, None),
    );

    let metric_name = "processes_blocked_current";
    let mut metric_value_string = String::from("NaN");
    if let Some(metric_value) = &(*topo).read_nb_process_blocked_current() {
        metric_value_string = metric_value.to_string();
    }
    body = push_metric(
        body,
        String::from("Number of processes currently blocked waiting for I/O."),
        String::from("gauge"),
        String::from(metric_name),
        format_metric(metric_name, &metric_value_string, None),
    );
    let metric_name = "context_switches_total";
    let mut metric_value_string = String::from("NaN");
    if let Some(metric_value) = &(*topo).read_nb_context_switches_total_count() {
        metric_value_string = metric_value.to_string();
    }
    body = push_metric(
        body,
        String::from("Number of context switches since boot."),
        String::from("counter"),
        String::from(metric_name),
        format_metric(metric_name, &metric_value_string, None),
    );

    let processes_tracker = &(*topo).proc_tracker;

    for pid in processes_tracker.get_alive_pids() {
        let exe = processes_tracker.get_process_name(pid);
        let cmdline = processes_tracker.get_process_cmdline(pid);

        let mut plabels = HashMap::new();
        plabels.insert(String::from("pid"), pid.to_string());
        plabels.insert(String::from("exe"), exe);
        if let Some(cmdline_str) = cmdline {
            //if cmdline_str.len() > 350 {
            //    cmdline_str = String::from(&cmdline_str[..350]);
            //}
            plabels.insert(String::from("cmdline"), cmdline_str.replace("\"", "\\\""));
        }

        let metric_name = "process_power_consumption_microwatts";
        let mut process_power_value = String::from("0");
        if let Some(power) = topo.get_process_power_consumption_microwatts(pid) {
            process_power_value = power.to_string();
        }
        body = push_metric(
            body, "Power consumption due to the process, measured on at the topology level, in microwatts".to_string(),
            String::from("gauge"), String::from(metric_name),
            format_metric (
                metric_name, &process_power_value,
                Some(&plabels)
            )
        );
    }

    HttpResponse::Ok()
        //.set_header("X-TEST", "value")
        .body(body)
}

/// Handles requests that are not asking for /metrics and returns the appropriate path in the body of the response.
async fn landing_page() -> impl Responder {
    let body = String::from(
        "<a href=\"https://github.com/hubblo-org/scaphandre/\">Scaphandre's</a> prometheus exporter here. Metrics available on <a href=\"/metrics\">/metrics</a>"
    );
    HttpResponse::Ok()
        //.set_header("X-TEST", "value")
        .body(body)
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
