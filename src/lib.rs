mod sensors;
use sensors::{powercap_rapl::PowercapRAPLSensor};
mod exporters;
use exporters::{Exporter, ExporterOption, stdout::StdoutExporter};
use clap::ArgMatches;
use std::collections::HashMap;

pub fn run(matches: ArgMatches) {
    let sensor = match matches.value_of("sensor").unwrap() {
        "powercap_rapl" => PowercapRAPLSensor::new(),
        _ => PowercapRAPLSensor::new()
    };
    let sensor_boxed = Box::new(sensor);

    let exporter_required = matches.subcommand_matches("stdout");
    if exporter_required.is_some() {
        let exporter_required = exporter_required.unwrap();
        let mut exporter = StdoutExporter::new(
                sensor_boxed, String::from(exporter_required.value_of("timeout").unwrap())
            );
        exporter.run();
    } else {
        eprintln!("exporter is None");
    }
}

pub fn get_exporters_options() -> HashMap<String, HashMap<String, ExporterOption>> {
    let mut options = HashMap::new();
    options.insert(
        String::from("stdout"),
        exporters::stdout::StdoutExporter::get_options()
    );
    options
}