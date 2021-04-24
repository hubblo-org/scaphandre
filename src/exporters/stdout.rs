use clap::Arg;

use crate::exporters::*;
use crate::sensors::{Record, Sensor, Topology};
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
            .help("Maximum time spent measuring, in seconds.")
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
        let timeout = parameters.value_of("timeout").unwrap();
        if timeout.is_empty() {
            self.iterate();
        } else {
            let now = Instant::now();

            let timeout_secs: u64 = timeout.parse().unwrap();

            // We have a default value of 2s so it is safe to unwrap the option
            // Panic if a non numerical value is passed
            let step_duration: u64 = parameters
                .value_of("step_duration")
                .unwrap()
                .parse()
                .expect("Wrong step_duration value, should be a number of seconds");

            println!("Measurement step is: {}s", step_duration);

            while now.elapsed().as_secs() <= timeout_secs {
                self.iterate();
                thread::sleep(Duration::new(step_duration, 0));
            }
        }
    }

    fn get_domains_power(&self, socket_id: u16) -> Vec<Option<Record>> {
        let socket_present = self
            .topology
            .get_sockets_passive()
            .iter()
            .find(move |x| x.id == socket_id);

        if let Some(socket) = socket_present {
            let mut domains_power: Vec<Option<Record>> = vec![];
            for d in socket.get_domains_passive() {
                domains_power.push(d.get_records_diff_power_microwatts());
            }
            domains_power
        } else {
            vec![None, None, None]
        }
    }

    fn iterate(&mut self) {
        self.topology.refresh();
        self.show_metrics();
    }

    fn show_metrics(&self) {
        let host_power = match self.topology.get_records_diff_power_microwatts() {
            Some(record) => record.value.parse::<u64>().unwrap(),
            None => 0,
        };
        let mut sockets_power: HashMap<u16, (u64, Vec<Option<Record>>)> = HashMap::new();
        let sockets = self.topology.get_sockets_passive();
        for s in sockets {
            let socket_power = match s.get_records_diff_power_microwatts() {
                Some(record) => record.value.parse::<u64>().unwrap(),
                None => 0,
            };
            sockets_power.insert(s.id, (socket_power, self.get_domains_power(s.id)));
        }
        println!(
            "Host:\t{} W\tCore\t\tUncore\t\tDRAM",
            (host_power as f32 / 1000000.0)
        );
        for (s_id, v) in sockets_power.iter() {
            let power = (v.0 as f32) / 1000000.0;
            let mut power_str = String::from('?');
            if power > 0.0 {
                power_str = power.to_string();
            }
            let mut to_print = format!("Socket{}\t{} W\t", s_id, power_str);
            for d in v.1.iter() {
                let domain_power = match d {
                    Some(record) => record.value.parse::<u64>().unwrap(),
                    None => 0,
                };
                to_print.push_str(&format!("{} W\t", domain_power as f32 / 1000000.0));
            }
            println!("{}", to_print);
        }
        println!("Top 5 consumers:");
        println!("Power\tPID\tExe");

        let consumers = self.topology.proc_tracker.get_top_consumers(5);
        for c in consumers.iter() {
            if let Some(host_stat) = self.topology.get_stats_diff() {
                let host_time = host_stat.total_time_jiffies();
                println!(
                    "{} W\t{}\t{:?}",
                    ((c.1 as f32 / (host_time * procfs::ticks_per_second().unwrap() as f32))
                        * host_power as f32)
                        / 1000000.0,
                    c.0.pid,
                    c.0.exe().unwrap_or_default()
                );
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
