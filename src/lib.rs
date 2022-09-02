//! Scaphandre is an extensible monitoring agent for energy consumption metrics.
//!
//! It gathers energy consumption data from the system or other data sources thanks to components called *sensors*.
//!
//! Final monitoring data is sent to or exposed for monitoring tools thanks to *exporters*.
#[macro_use]
extern crate log;
pub mod exporters;
pub mod sensors;
use clap::ArgMatches;
use colored::*;
#[cfg(feature = "warp10")]
use exporters::warpten::Warp10Exporter;
use exporters::{
    json::JSONExporter, prometheus::PrometheusExporter, qemu::QemuExporter,
    riemann::RiemannExporter, stdout::StdoutExporter, Exporter,
};
use sensors::{powercap_rapl::PowercapRAPLSensor, Sensor};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Helper function to get an argument from ArgMatches
fn get_argument(matches: &ArgMatches, arg: &'static str) -> String {
    if let Some(value) = matches.value_of(arg) {
        return String::from(value);
    }
    panic!("Couldn't get argument {}", arg);
}

/// Helper function to get a Sensor instance from ArgMatches
fn get_sensor(matches: &ArgMatches) -> Box<dyn Sensor> {
    let sensor = match &get_argument(matches, "sensor")[..] {
        "powercap_rapl" => PowercapRAPLSensor::new(
            get_argument(matches, "sensor-buffer-per-socket-max-kB")
                .parse()
                .unwrap(),
            get_argument(matches, "sensor-buffer-per-domain-max-kB")
                .parse()
                .unwrap(),
            matches.is_present("vm"),
        ),
        _ => PowercapRAPLSensor::new(
            get_argument(matches, "sensor-buffer-per-socket-max-kB")
                .parse()
                .unwrap(),
            get_argument(matches, "sensor-buffer-per-domain-max-kB")
                .parse()
                .unwrap(),
            matches.is_present("vm"),
        ),
    };
    Box::new(sensor)
}

macro_rules! declare_exporters {
    ($header:tt, $exporter_match_flag:tt, $matches:tt, $($name:tt, $exporter:ty,)+) => {$(
        if let Some(exporter_parameters) = $matches.subcommand_matches($name) {
            $exporter_match_flag = true;
            if $header {
                scaphandre_header($name);
            }
            let mut exporter = <$exporter>::new(get_sensor(&$matches)); // FIXME
            exporter.run(exporter_parameters.clone());
    }
    )+}
}

/// Matches the sensor and exporter name and options requested from the command line and
/// creates the appropriate instances. Launchs the standardized entrypoint of
/// the choosen exporter: run()
/// This function should be updated to take new exporters into account.
pub fn run(matches: ArgMatches) {
    loggerv::init_with_verbosity(matches.occurrences_of("v")).unwrap();

    let mut header = true;
    let mut exporter_match_flag = false;
    if matches.is_present("no-header") {
        header = false;
    }

    #[cfg(not(feature = "warp10"))]
    declare_exporters!(
        header,
        exporter_match_flag,
        matches,
        "stdout",
        StdoutExporter,
        "json",
        JSONExporter,
        "riemann",
        RiemannExporter,
        "prometheus",
        PrometheusExporter,
        "qemu",
        QemuExporter,
    );
    #[cfg(feature = "warp10")]
    declare_exporters!(
        header,
        exporter_match_flag,
        matches,
        "stdout",
        StdoutExporter,
        "json",
        JSONExporter,
        "riemann",
        RiemannExporter,
        "prometheus",
        PrometheusExporter,
        "qemu",
        QemuExporter,
        "warp10",       // <-- Added
        Warp10Exporter, // <-- Added
    );
    if !exporter_match_flag {
        error!("Couldn't determine which exporter has been chosen.");
    }
}

/// Returns options needed for each exporter as a HashMap.
/// This function has to be updated to enable a new exporter.
pub fn get_exporters_options() -> HashMap<String, Vec<clap::Arg<'static, 'static>>> {
    let mut options = HashMap::new();
    options.insert(
        String::from("stdout"),
        exporters::stdout::StdoutExporter::get_options(),
    );
    options.insert(
        String::from("json"),
        exporters::json::JSONExporter::get_options(),
    );
    options.insert(
        String::from("prometheus"),
        exporters::prometheus::PrometheusExporter::get_options(),
    );
    options.insert(
        String::from("riemann"),
        exporters::riemann::RiemannExporter::get_options(),
    );
    options.insert(
        String::from("qemu"),
        exporters::qemu::QemuExporter::get_options(),
    );
    #[cfg(feature = "warp10")]
    {
        options.insert(
            String::from("warp10"),
            exporters::warpten::Warp10Exporter::get_options(),
        );
    }

    options
}

fn current_system_time_since_epoch() -> Duration {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
}

pub fn scaphandre_header(exporter_name: &str) {
    let title = format!("Scaphandre {} exporter", exporter_name);
    println!("{}", title.red().bold());
    println!("Sending ⚡ metrics");
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
