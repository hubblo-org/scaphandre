#[macro_use] extern crate log;
use loggerv;
pub mod sensors;
pub mod exporters;
use sensors::{powercap_rapl::PowercapRAPLSensor};
use exporters::{Exporter, ExporterOption, stdout::StdoutExporter, prometheus::PrometheusExporter};
pub use clap::ArgMatches;
use std::collections::HashMap;

/// Matches the sensor and exporter name and options requested from the command line and
/// creates the appropriate instances. Launchs the standardized entrypoint of
/// the choosen exporter: run()
/// This function should be updated to take new exporters and sensors into account.
pub fn run(matches: ArgMatches) {

    loggerv::init_with_verbosity(matches.occurrences_of("v")).unwrap();

    let sensor = match matches.value_of("sensor").unwrap() {
        "powercap_rapl" => PowercapRAPLSensor::new(
            matches.value_of("sensor-buffer-per-socket-max-kB").unwrap().parse().unwrap(),
            matches.value_of("sensor-buffer-per-domain-max-kB").unwrap().parse().unwrap()
        ),
        _ => PowercapRAPLSensor::new(
            matches.value_of("sensor-buffer-per-socket-max-kB").unwrap().parse().unwrap(),
            matches.value_of("sensor-buffer-per-domain-max-kB").unwrap().parse().unwrap()
        )
    };
    let sensor_boxed = Box::new(sensor);

    let stdout_exporter_required = matches.subcommand_matches("stdout");
    if stdout_exporter_required.is_some() {
        let exporter_parameters = stdout_exporter_required.unwrap().clone();
        let mut exporter = StdoutExporter::new(sensor_boxed);
        exporter.run(exporter_parameters);
    } else {
        let prometheus_exporter_required = matches.subcommand_matches("prometheus");
        if prometheus_exporter_required.is_some() {
            let exporter_parameters = prometheus_exporter_required.unwrap().clone();
            let mut exporter = PrometheusExporter::new(sensor_boxed);
            exporter.run(exporter_parameters);
        } else {
            error!("Couldn't determine which exporter has been choosed.");
        }
    }
}

/// Returns options needed for each exporter as a HashMap.
/// This function has to be updated to enable a new exporter.
pub fn get_exporters_options() -> HashMap<String, HashMap<String, ExporterOption>> {
    let mut options = HashMap::new();
    options.insert(
        String::from("stdout"),
        exporters::stdout::StdoutExporter::get_options()
    );
    options.insert(
        String::from("prometheus"),
        exporters::prometheus::PrometheusExporter::get_options()
    );
    options
}