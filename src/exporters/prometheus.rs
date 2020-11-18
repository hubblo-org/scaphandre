use crate::exporters::{Exporter, ExporterOption};
use crate::sensors::{Sensor, Record, Topology, RecordGenerator};
use std::collections::HashMap;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use std::sync::Mutex;

/// Exporter that exposes metrics to an HTTP endpoint
/// matching the Prometheus.io metrics format.
pub struct PrometheusExporter {
    /// Sensor instance that is used to generate the Topology and
    /// thus get power consumption metrics.
    sensor: Box<dyn Sensor>,
    _step: u16
}

impl PrometheusExporter {
    /// Instantiates PrometheusExporter and returns the instance.
    pub fn new(sensor: Box<dyn Sensor>, step: u16) -> PrometheusExporter {
        PrometheusExporter{
            sensor: sensor,
            _step: step
        }
    }
}

impl Exporter for PrometheusExporter {
    /// Runs HTTP server and metrics exposure through the runner function.
    fn run(&mut self) {
        match runner((*self.sensor.get_topology()).unwrap()) {
            Ok(()) => warn!("Prometheus exporter shut down gracefully."),
            Err(error) => panic!("Something failed in the prometheus exporter: {}", error)
        }
    }
    /// Returns options understood by the exporter.
    fn get_options() -> HashMap<String, ExporterOption> {
        let options = HashMap::new();

        options
    }
}

/// Contains a mutex holding a Topology object. 
/// Used to pass the topology data from one http worker to another. 
pub struct PowerMetrics {
    topology: Mutex<Topology>
}

#[actix_web::main]
/// Main function running the HTTP server.
async fn runner(topology: Topology) -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new().data(
            PowerMetrics{
                topology: Mutex::new(topology.clone())
            }).service(
                web::resource("/metrics").route(
                    web::get().to(show_metrics)
                )
            ).default_service(
                web::route().to(landing_page)
            )
    }).workers(1).bind("0.0.0.0:8080")?
    .run().await
}

fn format_metric(key: &str, value: &str, labels: Option<&HashMap<String, String>>) -> String {
    let prefix = "scaph";
    let mut result = format!("{}_{}", prefix, key);
    if let Some(labels) = labels {
        result.push('{');
        for (k,v) in labels.iter() {
            result.push_str(&format!("{}=\"{}\",", k, v));
        }
        result.remove(result.len()-1);
        result.push('}');
    }
    result.push(' ');
    result.push_str(value);
    result.push_str("\n");
    result
}
fn push_metric(mut body: String, help: String, metric_type: String, metric_name: String, metric_line: String) -> String {
    body.push_str(&format!("# HELP {} {}", metric_name, help));
    body.push_str("\n");
    body.push_str(&format!("# TYPE {} {}", metric_name, metric_type));
    body.push_str("\n");
    body.push_str(&metric_line);
    body
}

async fn show_metrics(data: web::Data<PowerMetrics>) -> impl Responder {
    {
        let mut topology = data.topology.lock().unwrap();
        (*topology).refresh();
    }
    let topo = data.topology.lock().unwrap();
    let records = (*topo).get_records_passive();
    let mut body = String::from(""); // initialize empty body
    let mut rapl_host_energy_microjoules = String::from("NaN");
    let mut rapl_host_energy_timestamp_seconds = String::from("NaN");
    if !records.is_empty() {
        let record = records.last().unwrap();
        rapl_host_energy_microjoules = record.value.clone();
        rapl_host_energy_timestamp_seconds = record.timestamp.as_secs().to_string();
    }

    let metric_name = "rapl_host_energy_microjoules";
    body = push_metric(
        body, String::from("Energy measurement for the whole host, as extracted from the sensor, in microjoules."),
        String::from("counter"),
        String::from(metric_name),
        format_metric(
            metric_name,
            &rapl_host_energy_microjoules, 
            None
        )
    );

    let metric_name = "rapl_host_energy_timestamp_seconds";
    body = push_metric(
        body,String::from("Timestamp in seconds when rapl_hose_energy_microjoules has been computed."),
        String::from("counter"),
        String::from(metric_name),
        format_metric(
            metric_name,
            &rapl_host_energy_timestamp_seconds, 
            None
        )
    );

    let mut rapl_host_power_microwatts = "Nan";
    let host_power_record: Record;
    if let Some(power) = (*topo).get_records_diff_power_microwatts() {
        host_power_record = power;
        rapl_host_power_microwatts = &host_power_record.value;
    }

    let metric_name = "rapl_host_power_microwatts";
    body = push_metric(
        body, 
        String::from("Power measurement on the whole host, in microwatts"),
        String::from("gauge"),
        String::from(metric_name), 
        format_metric(
            metric_name, 
            rapl_host_power_microwatts, 
            None
        )
    );

    let sockets = (*topo).get_sockets_passive();
    for s in sockets {
        let records = s.get_records_passive();
        let mut rapl_socket_energy_microjoules = "NaN";
        if !records.is_empty() {
            rapl_socket_energy_microjoules = &records.last().unwrap().value;
        }
        let mut labels = HashMap::new();
        labels.insert(String::from("socket_id"), s.id.to_string());
        
        let metric_name = "rapl_socket_energy_microjoules";
        body = push_metric(
            body, String::from("Socket related energy measurement in mirojoules."),
            String::from("counter"), 
            String::from(metric_name),
            format_metric(
                metric_name,
                rapl_socket_energy_microjoules,
                Some(&labels)
            )
        );
        let mut rapl_socket_power_microwatts = "NaN";
        let socket_power_record: Record;
        if let Some(power) = (*topo).get_records_diff_power_microwatts() {
            socket_power_record = power;
            rapl_socket_power_microwatts = &socket_power_record.value;
        }
        
        let metric_name = "socket_power_microwatts";
        body = push_metric(
            body, String::from("Power measurement relative to a CPU socket, in microwatts"),
            String::from("gauge"),
            String::from(metric_name),
            format_metric(
                metric_name,
                rapl_socket_power_microwatts, 
                Some(&labels)
            )
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
        body, String::from(
            "Number of processes currently running."
        ),
        String::from("gauge"),
        String::from(metric_name),
        format_metric(
            metric_name, &metric_value_string, None
        )
    );

    let metric_name = "processes_blocked_current";
    let mut metric_value_string = String::from("NaN");
    if let Some(metric_value) = &(*topo).read_nb_process_blocked_current() {
        metric_value_string = metric_value.to_string();
    }
    body = push_metric(
        body, String::from(
            "Number of processes currently blocked waiting for I/O."
        ),
        String::from("gauge"),
        String::from(metric_name),
        format_metric(
            metric_name, &metric_value_string, None
        )
    );
    let metric_name = "context_switches_total";
    let mut metric_value_string = String::from("NaN");
    if let Some(metric_value) = &(*topo).read_nb_context_switches_total_count() {
        metric_value_string = metric_value.to_string();
    }
    body = push_metric(
        body, String::from(
            "Number of context switches since boot."
        ),
        String::from("counter"),
        String::from(metric_name),
        format_metric(
            metric_name, &metric_value_string, None
        )
    );


    let processes_tracker = &(*topo).proc_tracker;

    for pid in processes_tracker.get_all_pids() {
        let exe = processes_tracker.get_process_name(pid);
        let cmdline = processes_tracker.get_process_cmdline(pid);

        let mut plabels = HashMap::new();
        plabels.insert(String::from("pid"), pid.to_string());
        plabels.insert(String::from("exe"), exe);
        if cmdline.is_some() {
            plabels.insert(String::from("cmdline"), cmdline.unwrap());
        }

        //let mut stime = String::from("NaN");
        //if let Some(res) = processes_tracker.get_diff_stime(pid) {
        //    stime = res.to_string();
        //}

        //let metric_name = "process_stime_jiffies";
        //body = push_metric(
        //    body, String::from(
        //        format!("System time consumed on the CPU by pid {}, in jiffies (relative to CPU model).", pid)
        //    ),
        //    String::from("counter"),
        //    String::from(metric_name),
        //    format_metric(
        //        metric_name, &stime, Some(&plabels)
        //    )
        //);

        //let mut utime = String::from("NaN");
        //if let Some(res) = processes_tracker.get_diff_utime(pid) {
        //    utime = res.to_string();
        //}

        //let metric_name = "process_utime_jiffies";
        //body = push_metric(
        //    body, String::from(
        //        format!("User time consumed on the CPU by pid {}, in jiffies (relative to CPU model).", pid)
        //    ),
        //    String::from("counter"),
        //    String::from(metric_name),
        //    format_metric(
        //        metric_name, &utime, Some(&plabels)
        //    )            
        //);

        let metric_name = "process_power_consumption_microwatts";
        let mut process_power_value = String::from("0");
        if let Some(power) = topo.get_process_power_consumption_microwatts(pid) {
           process_power_value = power.to_string(); 
        }
        body = push_metric(
            body, String::from(format!("Power consumption due to the process, measured on at the topology level, in microwatts")),
            String::from("gauge"), String::from(metric_name),
            format_metric (
                metric_name, &process_power_value,
                Some(&plabels)
            )
        );
    }

    HttpResponse::Ok()
        //.set_header("X-TEST", "value")
        .body(
           body 
        )
}

async fn landing_page() -> impl Responder {
    let body = String::from(
        "<a href=\"https://github.com/hubblo-org/scaphandre/\">Scaphandre's</a> prometheus exporter here. Metrics available on <a href=\"/metrics\">/metrics</a>"
    );
    HttpResponse::Ok()
        //.set_header("X-TEST", "value")
        .body(
           body 
        )
}