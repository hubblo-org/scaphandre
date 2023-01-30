use clap::Arg;

use crate::exporters::*;
use crate::sensors::{utils::IProcess, Sensor};
use colored::*;
use regex::Regex;
use std::fmt::Write as _;
use std::thread;
use std::time::{Duration, Instant};

/// An Exporter that displays power consumption data of the host
/// and its processes on the standard output of the terminal.
pub struct StdoutExporter {
    sensor: Box<dyn Sensor>,
}

impl Exporter for StdoutExporter {
    /// Lanches runner()
    fn run(&mut self, parameters: ArgMatches) {
        self.runner(parameters);
    }

    /// Returns options needed for that exporter, as a HashMap
    fn get_options() -> Vec<clap::Arg<'static, 'static>> {
        let mut options = Vec::new();
        let arg = Arg::with_name("timeout")
            .default_value("10")
            .help("Maximum time spent measuring, in seconds. 0 means continuous measurement.")
            .long("timeout")
            .short("t")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("step_duration")
            .default_value("2")
            .help("Set measurement step duration in second.")
            .long("step")
            .short("s")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("process_number")
            .default_value("5")
            .help("Number of processes to display.")
            .long("process")
            .short("p")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("regex_filter")
            .help("Filter processes based on regular expressions (e.g: 'scaph\\w\\wd.e'). This option disable '-p' or '--process' one.")
            .long("regex")
            .short("r")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("qemu")
            .help("Apply labels to metrics of processes looking like a Qemu/KVM virtual machine")
            .long("qemu")
            .short("q")
            .required(false)
            .takes_value(false);
        options.push(arg);

        options
    }
}

impl StdoutExporter {
    /// Instantiates and returns a new StdoutExporter
    pub fn new(sensor: Box<dyn Sensor>) -> StdoutExporter {
        StdoutExporter { sensor }
    }

    /// Runs iteration() every 'step', during until 'timeout'
    pub fn runner(&mut self, parameters: ArgMatches) {
        // Parse parameters
        // All parameters have a default values so it is safe to unwrap them.
        // Panic if a non numerical value is passed except for regex_filter.

        let timeout_secs: u64 = parameters
            .value_of("timeout")
            .unwrap()
            .parse()
            .expect("Wrong timeout value, should be a number of seconds");

        let step_duration: u64 = parameters
            .value_of("step_duration")
            .unwrap()
            .parse()
            .expect("Wrong step_duration value, should be a number of seconds");

        let process_number: u16 = parameters
            .value_of("process_number")
            .unwrap()
            .parse()
            .expect("Wrong process_number value, should be a number");

        let regex_filter: Option<Regex> = if !parameters.is_present("regex_filter")
            || parameters.value_of("regex_filter").unwrap().is_empty()
        {
            None
        } else {
            Some(
                Regex::new(parameters.value_of("regex_filter").unwrap())
                    .expect("Wrong regex_filter, regexp is invalid"),
            )
        };

        if parameters.occurrences_of("regex_filter") == 1
            && parameters.occurrences_of("process_number") == 1
        {
            let warning =
                String::from("Warning: (-p / --process) and (-r / --regex) used at the same time. (-p / --process) disabled");
            eprintln!("{}", warning.bright_yellow());
        }

        let topology = self.sensor.get_topology().unwrap();
        let mut metric_generator = MetricGenerator::new(
            topology,
            utils::get_hostname(),
            parameters.is_present("qemu"),
            parameters.is_present("containers"),
        );

        println!("Measurement step is: {step_duration}s");
        if timeout_secs == 0 {
            loop {
                self.iterate(&regex_filter, process_number, &mut metric_generator);
                thread::sleep(Duration::new(step_duration, 0));
            }
        } else {
            let now = Instant::now();

            while now.elapsed().as_secs() <= timeout_secs {
                self.iterate(&regex_filter, process_number, &mut metric_generator);
                thread::sleep(Duration::new(step_duration, 0));
            }
        }
    }

    fn iterate(
        &mut self,
        regex_filter: &Option<Regex>,
        process_number: u16,
        metric_generator: &mut MetricGenerator,
    ) {
        metric_generator
            .topology
            .proc_tracker
            .clean_terminated_process_records_vectors();
        metric_generator.topology.refresh();
        self.show_metrics(regex_filter, process_number, metric_generator);
    }

    fn show_metrics(
        &self,
        regex_filter: &Option<Regex>,
        process_number: u16,
        metric_generator: &mut MetricGenerator,
    ) {
        metric_generator.gen_all_metrics();

        let metrics = metric_generator.pop_metrics();
        let mut metrics_iter = metrics.iter();
        let host_power = match metrics_iter.find(|x| x.name == "scaph_host_power_microwatts") {
            Some(m) => m.metric_value.clone(),
            None => MetricValueType::Text("0".to_string()),
        };

        let domain_names = metric_generator.topology.domains_names.as_ref();
        if domain_names.is_some() {
            info!("domain_names: {:?}", domain_names.unwrap());
        }

        println!(
            "Host:\t{} W",
            (format!("{host_power}").parse::<f64>().unwrap() / 1000000.0)
        );

        if domain_names.is_some() {
            println!("\tpackage \t{}", domain_names.unwrap().join("\t\t"));
        }

        for s in metrics
            .iter()
            .filter(|x| x.name == "scaph_socket_power_microwatts")
        {
            let power = format!("{}", s.metric_value).parse::<f32>().unwrap() / 1000000.0;
            let mut power_str = String::from("----");
            if power > 0.0 {
                power_str = power.to_string();
            }
            let socket_id = s.attributes.get("socket_id").unwrap().clone();

            let mut to_print = format!("Socket{socket_id}\t{power_str} W |\t");

            let domains = metrics.iter().filter(|x| {
                x.name == "scaph_domain_power_microwatts"
                    && x.attributes.get("socket_id").unwrap() == &socket_id
            });

            if let Some(domain_names) = domain_names {
                for d in domain_names {
                    info!("current domain : {}", d);
                    info!("domains size : {}", &domains.clone().count());
                    if let Some(current_domain) = domains.clone().find(|x| {
                        info!("looking for domain metrics for d == {}", d);
                        info!("current metric analyzed : {:?}", x);
                        if let Some(domain_name_result) = x.attributes.get("domain_name") {
                            if domain_name_result == d {
                                return true;
                            }
                        }
                        false
                    }) {
                        let _ = write!(
                            to_print,
                            "{} W\t",
                            current_domain
                                .metric_value
                                .to_string()
                                .parse::<f32>()
                                .unwrap()
                                / 1000000.0
                        );
                    } else {
                        to_print.push_str("---");
                    }
                }
                println!("{to_print}\n");
            }
        }

        let consumers: Vec<(IProcess, f64)>;
        if let Some(regex_filter) = regex_filter {
            println!("Processes filtered by '{}':", regex_filter.as_str());
            consumers = metric_generator
                .topology
                .proc_tracker
                .get_filtered_processes(regex_filter);
        } else {
            println!("Top {process_number} consumers:");
            consumers = metric_generator
                .topology
                .proc_tracker
                .get_top_consumers(process_number);
        }

        info!("consumers : {:?}", consumers);
        println!("Power\t\tPID\tExe");
        if consumers.is_empty() {
            println!("No processes found yet or filter returns no value.");
        } else {
            for c in consumers.iter() {
                if let Some(process) = metrics.iter().find(|x| {
                    if x.name == "scaph_process_power_consumption_microwatts" {
                        let pid = x.attributes.get("pid").unwrap();
                        pid.parse::<i32>().unwrap() == c.0.pid
                    } else {
                        false
                    }
                }) {
                    println!(
                        "{} W\t{}\t{:?}",
                        format!("{}", process.metric_value).parse::<f32>().unwrap() / 1000000.0,
                        process.attributes.get("pid").unwrap(),
                        process.attributes.get("exe").unwrap()
                    );
                }
            }
        }
        println!("------------------------------------------------------------\n");
    }
}

#[cfg(test)]
mod tests {
    //#[test]
    //fn get_cons_socket0() {}
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
