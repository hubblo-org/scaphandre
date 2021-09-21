use clap::Arg;

use crate::exporters::*;
use crate::sensors::{Record, Sensor, Topology};
use colored::*;
use regex::Regex;
use std::collections::HashMap;
use std::thread;
use std::time::{Duration, Instant};

/// An Exporter that displays power consumption data of the host
/// and its processes on the standard output of the terminal.
pub struct StdoutExporter {
    topology: Topology,
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
    pub fn new(mut sensor: Box<dyn Sensor>) -> StdoutExporter {
        let some_topology = *sensor.get_topology();
        StdoutExporter {
            topology: some_topology.unwrap(),
        }
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

        let regex_filter: Option<Regex>;
        if !parameters.is_present("regex_filter")
            || parameters.value_of("regex_filter").unwrap().is_empty()
        {
            regex_filter = None;
        } else {
            regex_filter = Some(
                Regex::new(parameters.value_of("regex_filter").unwrap())
                    .expect("Wrong regex_filter, regexp is invalid"),
            );
        }

        if parameters.occurrences_of("regex_filter") == 1
            && parameters.occurrences_of("process_number") == 1
        {
            let warning =
                String::from("Warning: (-p / --process) and (-r / --regex) used at the same time. (-p / --process) disabled");
            eprintln!("{}", warning.bright_yellow());
        }

        println!("Measurement step is: {}s", step_duration);
        if timeout_secs == 0 {
            loop {
                self.iterate(&regex_filter, process_number, parameters.is_present("qemu"));
                thread::sleep(Duration::new(step_duration, 0));
            }
        } else {
            let now = Instant::now();

            while now.elapsed().as_secs() <= timeout_secs {
                self.iterate(&regex_filter, process_number, parameters.is_present("qemu"));
                thread::sleep(Duration::new(step_duration, 0));
            }
        }
    }

    // Retuns the power for each domain in a socket.
    fn get_domains_power(&self, socket_id: u16) -> HashMap<String, Option<Record>> {
        let socket_present = self
            .topology
            .get_sockets_passive()
            .iter()
            .find(move |x| x.id == socket_id);

        if let Some(socket) = socket_present {
            // let mut domains_power: Vec<Option<Record>> = vec![];
            let mut domains_power: HashMap<String, Option<Record>> = HashMap::new();
            for d in socket.get_domains_passive() {
                domains_power.insert(d.name.clone(), d.get_records_diff_power_microwatts());
            }
            domains_power
        } else {
            HashMap::new()
        }
    }

    fn iterate(&mut self, regex_filter: &Option<Regex>, process_number: u16, qemu: bool) {
        self.topology.refresh();
        self.show_metrics(regex_filter, process_number, qemu);
    }

    fn show_metrics(&self, regex_filter: &Option<Regex>, process_number: u16, qemu: bool) {
        let hostname = utils::get_hostname();
        let mut metric_generator = MetricGenerator::new(&self.topology, &hostname);
        metric_generator.gen_all_metrics(qemu);

        let metrics = metric_generator.get_metrics();
        let mut metrics_iter = metrics.iter();
        let host_power =  match metrics_iter.find(|x| x.name == "scaph_host_power_microwatts") {
            Some(m) => m.metric_value.clone(),
            None => MetricValueType::Text("0".to_string())
        };
        //let host_power = match self.topology.get_records_diff_power_microwatts() {
        //    Some(record) => record.value.parse::<u64>().unwrap(),
        //    None => 0,
        //};

        let mut sockets_power: HashMap<u16, (u64, HashMap<String, Option<Record>>)> =
            HashMap::new();
        let sockets = self.topology.get_sockets_passive();
        for s in sockets {
            let socket_power = match s.get_records_diff_power_microwatts() {
                Some(record) => record.value.parse::<u64>().unwrap(),
                None => 0,
            };
            sockets_power.insert(s.id, (socket_power, self.get_domains_power(s.id)));
        }
        let domain_names = self.topology.domains_names.as_ref().unwrap();

        println!("Host:\t{} W", (format!("{}", host_power).parse::<f64>().unwrap() / 1000000.0));
        println!("\tpackage \t{}", domain_names.join("\t\t"));

        for (s_id, v) in sockets_power.iter() {
            let power = (v.0 as f32) / 1000000.0;
            let mut power_str = String::from("----");
            if power > 0.0 {
                power_str = power.to_string();
            }

            let mut to_print = format!("Socket{}\t{} W\t", s_id, power_str);

            for domain in domain_names.iter() {
                if let Some(Some(record)) = v.1.get(domain) {
                    to_print.push_str(&format!(
                        "{} W\t",
                        record.value.parse::<u64>().unwrap() as f32 / 1000000.0
                    ));
                } else {
                    // This should only happen when we don't have yet enough records,
                    // as in a multi-sockets system, all sockets have the same domains.
                    to_print.push_str("---- \t\t");
                }
            }
            println!("{}\n", to_print);
        }

        let consumers: Vec<(procfs::process::Process, u64)>;
        if let Some(regex_filter) = regex_filter {
            println!("Processes filtered by '{}':", regex_filter.as_str());
            consumers = self
                .topology
                .proc_tracker
                .get_filtered_processes(regex_filter);
        } else {
            println!("Top {} consumers:", process_number);
            consumers = self.topology.proc_tracker.get_top_consumers(process_number);
        }

        println!("Power\tPID\tExe");
        if consumers.is_empty() {
            println!("No processes found yet or filter returns no value.");
        } else {
            for c in consumers.iter() {
                if let Some(host_stat) = self.topology.get_stats_diff() {
                    let host_time = host_stat.total_time_jiffies();
                    println!(
                        "{} W\t{}\t{:?}",
                        ((c.1 as f32 / (host_time * procfs::ticks_per_second().unwrap() as f32))
                            * format!("{}", host_power).parse::<f32>().unwrap())
                            / 1000000.0,
                        c.0.pid,
                        c.0.exe().unwrap_or_default()
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
