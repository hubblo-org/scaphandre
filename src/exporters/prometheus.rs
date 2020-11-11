use crate::exporters::{Exporter, ExporterOption};
use crate::sensors::{Sensor, Topology, RecordGenerator, energy_records_to_power_record};
use std::collections::HashMap;
use actix_web::{get, web, App, HttpRequest, HttpServer, Responder};

pub struct PrometheusExporter {
    topology: Topology,
    step: u16
}

impl PrometheusExporter {
    pub fn new(mut sensor: Box<dyn Sensor>, step: u16) -> PrometheusExporter {
        PrometheusExporter{
            topology: (*sensor.get_topology()).unwrap(),
            step
        }
    }

    #[actix_web::main]
    async fn runner(&mut self) -> std::io::Result<()> {
        HttpServer::new( || {
            App::new().route("/metrics", web::get().to(show_metrics))
        }).bind("0.0.0.0:8080")?
        .run().await
    }

}

impl Exporter for PrometheusExporter {
    fn run(&mut self) {
        self.runner();
    }
    fn get_options() -> HashMap<String, ExporterOption> {
        let mut options = HashMap::new();

        options
    }
}

async fn show_metrics(req: HttpRequest) -> impl Responder {
    format!("HEY YOU")
}