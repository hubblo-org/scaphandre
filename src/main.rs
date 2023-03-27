//! Generic sensor and transmission agent for energy consumption related metrics.

use clap::{crate_authors, crate_version, value_parser, Arg, ArgAction, Command};
use scaphandre::{get_exporters_options, run};
fn main() {
    #[cfg(target_os = "linux")]
    let sensors = ["powercap_rapl"];
    #[cfg(target_os = "windows")]
    let sensors = ["msr_rapl"];
    let exporters_options = get_exporters_options();
    let exporters: Vec<String> = exporters_options.keys().map(|x| x.to_string()).collect();

    #[cfg(target_os = "linux")]
    let sensor_default_value = String::from("powercap_rapl");
    #[cfg(not(target_os = "linux"))]
    let sensor_default_value = String::from("msr_rapl");

    let mut matches = Command::new("scaphandre")
        .author(crate_authors!())
        .version(crate_version!())
        .about("Extensible metrology agent for energy/electricity consumption related metrics")
        .arg(
            Arg::new("v")
                .short('v')
                .help("Sets the level of verbosity.")
                .action(ArgAction::Count)
        )
        .arg(
            Arg::new("no-header")
                .value_name("no-header")
                .help("Prevents the header to be displayed in the terminal output.")
                .required(false)
                .long("no-header")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("sensor")
                .value_name("sensor")
                .help("Sensor module to apply on the host to get energy consumption metrics.")
                .required(false)
                .default_value(&sensor_default_value)
                .short('s')
                .long("sensor")
                .value_parser(sensors)
                .action(clap::ArgAction::Set)
        ).arg(
            Arg::new("sensor-buffer-per-domain-max-kB")
                .value_name("sensor-buffer-per-domain-max-kB")
                .help("Maximum memory size allowed, in KiloBytes, for storing energy consumption of each domain.")
                .required(false)
                .default_value("1")
                .value_parser(value_parser!(u16))
                .action(clap::ArgAction::Set)
        ).arg(
            Arg::new("sensor-buffer-per-socket-max-kB")
                .value_name("sensor-buffer-per-socket-max-kB")
                .help("Maximum memory size allowed, in KiloBytes, for storing energy consumption of each socket.")
                .required(false)
                .default_value("1")
                .value_parser(value_parser!(u16))
                .action(clap::ArgAction::Set)
        ).arg(
            Arg::new("vm")
                .value_name("vm")
                .help("Tell scaphandre if he is running in a virtual machine.")
                .long("vm")
                .required(false)
                .action(clap::ArgAction::SetTrue),
        );

    for exporter in exporters {
        let mut subcmd = Command::new(&exporter).about(
            match exporter.as_str() {
                "stdout" => "Stdout exporter allows you to output the power consumption data in the terminal",
                "json" => "JSON exporter allows you to output the power consumption data in a json file",
                "prometheus" => "Prometheus exporter exposes power consumption metrics on an http endpoint (/metrics is default) in prometheus accepted format",
                "riemann" => "Riemann exporter sends power consumption metrics to a Riemann server",
                "qemu" => "Qemu exporter watches all Qemu/KVM virtual machines running on the host and exposes metrics of each of them in a dedicated folder",
                "warp10" => "Warp10 exporter sends data to a Warp10 host, through HTTP",
                _ => "Unknown exporter",
            }
        );

        let myopts = exporters_options.get(&exporter).unwrap();
        for opt in myopts {
            subcmd = subcmd.arg(opt);
        }
        matches = matches.subcommand(subcmd);
    }
    run(matches.get_matches());
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
