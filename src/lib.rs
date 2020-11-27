#[macro_use] extern crate log;
use loggerv;
pub mod sensors;
pub mod exporters;
use sensors::{Sensor, powercap_rapl::PowercapRAPLSensor};
use exporters::{
    Exporter, ExporterOption, stdout::StdoutExporter, prometheus::PrometheusExporter,
    qemu::QemuExporter
};
pub use clap::ArgMatches;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};


/// Helper function to get an argument from ArgMatches
fn get_argument(matches: &ArgMatches, arg: &'static str) -> String {
    if let Some(value) = matches.value_of(arg) {
        return String::from(value)
    }
    panic!("Couldn't get argument {}", arg);
}

/// Helper function to get a Sensor instance from ArgMatches
fn get_sensor(matches: &ArgMatches) -> Box<dyn Sensor>{
    let sensor = match &get_argument(matches, "sensor")[..] {
        "powercap_rapl" => PowercapRAPLSensor::new(
            get_argument(matches,"sensor-buffer-per-socket-max-kB").parse().unwrap(),
            get_argument(matches, "sensor-buffer-per-domain-max-kB").parse().unwrap(),
            matches.is_present("vm")
        ),
        _ => PowercapRAPLSensor::new(
            get_argument(matches, "sensor-buffer-per-socket-max-kB").parse().unwrap(),
            get_argument(matches, "sensor-buffer-per-domain-max-kB").parse().unwrap(),
            matches.is_present("vm")
        )
    };
    Box::new(sensor)
}

/// Matches the sensor and exporter name and options requested from the command line and
/// creates the appropriate instances. Launchs the standardized entrypoint of
/// the choosen exporter: run()
/// This function should be updated to take new exporters into account.
pub fn run(matches: ArgMatches) {

    loggerv::init_with_verbosity(matches.occurrences_of("v")).unwrap();

    let sensor_boxed = get_sensor(&matches);

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
            let qemu_exporter_required = matches.subcommand_matches("qemu");
            if let Some(exporter_parameters) = qemu_exporter_required {
                let mut exporter = QemuExporter::new(sensor_boxed);
                exporter.run(exporter_parameters.clone());
            }
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
    options.insert(
        String::from("qemu"),
        exporters::qemu::QemuExporter::get_options()
    );
    options
}

pub fn current_system_time_since_epoch() -> Duration {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap()
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
