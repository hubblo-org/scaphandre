use crate::exporters::{Exporter, ExporterOption};
use crate::sensors::{Sensor, Topology, RecordGenerator, energy_records_to_power_record};
use std::collections::HashMap;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use std::sync::Mutex;

pub struct PrometheusExporter {
    sensor: Box<dyn Sensor>,
    step: u16
}

impl PrometheusExporter {
    pub fn new(mut sensor: Box<dyn Sensor>, step: u16) -> PrometheusExporter {
        PrometheusExporter{
            sensor: sensor,
            step: step
        }
    }

}


impl Exporter for PrometheusExporter {
    fn run(&mut self) {
        runner((*self.sensor.get_topology()).unwrap());
    }
    fn get_options() -> HashMap<String, ExporterOption> {
        let mut options = HashMap::new();

        options
    }
}

pub struct PowerMetrics {
    topology: Mutex<Topology>
}

#[actix_web::main]
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
async fn show_metrics(data: web::Data<PowerMetrics>) -> impl Responder {
    let mut topo = data.topology.lock().unwrap();
    (*topo).refresh();
    let records = (*topo).get_records_passive();
    format!("records: {:?}", records);
    let mut body = String::from("");
    for r in records.iter() {
        body.push_str(&format!("{}\n", r.value));
    }
    HttpResponse::Ok()
        .set_header("X-TEST", "value")
        .body(
           body 
        )
}

async fn landing_page() -> impl Responder {
    let body = String::from(
        "<a href=\"https://github.com/hubblo-org/scaphandre/\">Scaphandre's</a> prometheus exporter here. Metrics available on <a href=\"/metrics\">/metrics</a>"
    );
    HttpResponse::Ok()
        .set_header("X-TEST", "value")
        .body(
           body 
        )
}