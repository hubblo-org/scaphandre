//! Generic sensor and transmission agent for energy consumption related metrics.
use clap::{crate_version, App, AppSettings, Arg, SubCommand};
use scaphandre::{get_exporters_options, run};
fn main() {
    let sensors = ["powercap_rapl"];
    let exporters_options = get_exporters_options();
    let exporters = exporters_options.keys();
    let exporters: Vec<&str> = exporters.into_iter().map(|x| x.as_str()).collect();

    let mut matches = App::new("scaphandre")
        .author("Benoit Petit <bpetit@hubblo.org>")
        .version("0.3.0")
        .long_version(crate_version!())
        .about("Extensible metrology agent for energy/electricity consumption related metrics")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity.")
        )
        .arg(
            Arg::with_name("no-header")
                .value_name("no-header")
                .help("Prevents the header to be displayed in the terminal output.")
                .required(false)
                .takes_value(false)
                .long("no-header")
        )
        .arg(
            Arg::with_name("sensor")
                .value_name("sensor")
                .help("Sensor module to apply on the host to get energy consumption metrics.")
                .required(false)
                .takes_value(true)
                .default_value("powercap_rapl")
                .possible_values(&sensors)
                .short("s")
                .long("sensor")
        ).arg(
            Arg::with_name("sensor-buffer-per-domain-max-kB")
                .value_name("sensor-buffer-per-domain-max-kB")
                .help("Maximum memory size allowed, in KiloBytes, for storing energy consumption of each domain.")
                .required(false)
                .takes_value(true)
                .default_value("1")
        ).arg(
            Arg::with_name("sensor-buffer-per-socket-max-kB")
                .value_name("sensor-buffer-per-socket-max-kB")
                .help("Maximum memory size allowed, in KiloBytes, for storing energy consumption of each socket.")
                .required(false)
                .takes_value(true)
                .default_value("1")
        ).arg(
            Arg::with_name("vm")
                .value_name("vm")
                .help("Tell scaphandre if he is running in a virtual machine.")
                .long("vm")
                .required(false)
                .takes_value(false)
        );

    for exporter in exporters {
        let mut subcmd = SubCommand::with_name(exporter).about(
            match exporter {
                "stdout" => "Stdout exporter allows you to output the power consumption data in the terminal",
                "json" => "JSON exporter allows you to output the power consumption data in a json file",
                "prometheus" => "Prometheus exporter exposes power consumption metrics on an http endpoint (/metrics is default) in prometheus accepted format",
                "riemann" => "Riemann exporter sends power consumption metrics to a Riemann server",
                "qemu" => "Qemu exporter watches all Qemu/KVM virtual machines running on the host and exposes metrics of each of them in a dedicated folder",
                "warp10" => "Warp10 exporter sends data to a Warp10 host, through HTTP",
                _ => "Unknown exporter",
            }
        );

        let myopts = exporters_options.get(exporter).unwrap();
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
