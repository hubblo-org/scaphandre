mod sensors;
use sensors::{powercap_rapl::PowercapRAPLSensor};
mod exporters;
use exporters::{Exporter, stdout::StdoutExporter};
use clap::ArgMatches;

pub fn run(matches: ArgMatches) {
    let sensor = match matches.value_of("sensor").unwrap() {
        "powercap_rapl" => PowercapRAPLSensor::new(),
        _ => PowercapRAPLSensor::new()
    };
    let sensor_boxed = Box::new(sensor);

    let mut exporter = match matches.value_of("exporter").unwrap() {
        "stdout" => StdoutExporter::new(sensor_boxed),
        _ => StdoutExporter::new(sensor_boxed),
    };

    exporter.run();
}