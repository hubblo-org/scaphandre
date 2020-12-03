use crate::exporters::*;
use crate::sensors::{energy_records_to_power_record, RecordGenerator, Sensor, Topology};
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
    fn get_options() -> HashMap<String, ExporterOption> {
        let mut options = HashMap::new();
        options.insert(
            String::from("timeout"),
            ExporterOption {
                default_value: String::from("10"),
                long: String::from("timeout"),
                short: String::from("t"),
                required: false,
                takes_value: true,
                help: String::from("Maximum time spent measuring, in seconds."),
            },
        );
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
            self.iteration(0);
        } else {
            let now = Instant::now();

            let timeout_secs: u64 = timeout.parse().unwrap();
            let step = 1;

            println!("Measurement step is: {}s", step);

            while now.elapsed().as_secs() <= timeout_secs {
                self.iteration(step);
                thread::sleep(Duration::new(step, 0));
            }
        }
    }

    /// Processes and displays metrics
    fn iteration(&mut self, _step: u64) {
        self.topology.refresh();
        let topo_records = self.topology.get_records_passive();
        let mut topo_power = String::from("n/a");
        let mut topo_raw_power: f64 = 0.0;
        if topo_records.len() > 1 {
            topo_raw_power = energy_records_to_power_record((
                topo_records.last().unwrap(),
                topo_records.get(topo_records.len() - 2).unwrap(),
            ))
            .unwrap()
            .value
            .parse::<f64>()
            .unwrap();
            topo_power = topo_raw_power.to_string();
        }
        let (topo_stats_user, topo_stats_nice, topo_stats_system) =
            match self.topology.get_stats_diff() {
                Some(stat) => (
                    stat.cputime.user.to_string(),
                    stat.cputime.nice.to_string(),
                    stat.cputime.system.to_string(),
                ),
                None => (
                    String::from("n/a"),
                    String::from("n/a"),
                    String::from("n/a"),
                ),
            };
        for (counter, p) in self
            .topology
            .proc_tracker
            .get_alive_pids()
            .into_iter()
            .enumerate()
        {
            let utime_value = match self.topology.proc_tracker.get_diff_utime(p) {
                Some(time) => time.to_string(),
                None => String::from("n/a"),
            };
            let stime_value = match self.topology.proc_tracker.get_diff_stime(p) {
                Some(time) => time.to_string(),
                None => String::from("n/a"),
            };
            let mut utime_percent = 0.0;
            let mut stime_percent = 0.0;
            if topo_stats_system != "n/a"
                && topo_stats_user != "n/a"
                && utime_value != "n/a"
                && stime_value != "n/a"
            {
                utime_percent = utime_value.parse::<f64>().unwrap()
                    / topo_stats_user.parse::<f64>().unwrap()
                    * 100.0;
                stime_percent = stime_value.parse::<f64>().unwrap()
                    / topo_stats_system.parse::<f64>().unwrap()
                    * 100.0;
            }
            print!(
                "| {} utime:{} stime:{} utime_t_%:{} s_time_t_%: {} power: {}",
                p,
                utime_value,
                stime_value,
                utime_percent.to_string(),
                stime_percent.to_string(),
                ((utime_percent + stime_percent) * topo_raw_power / 100.0)
            );
            if counter % 4 == 0 {
                println!();
            }
        }
        for socket in self.topology.get_sockets() {
            let socket_id = socket.id;
            let socket_records = socket.get_records_passive();
            let mut power = String::from("unknown");
            let mut unit = String::from("W");
            let nb_records = socket_records.len();
            if nb_records > 1 {
                let power_record = &energy_records_to_power_record((
                    socket_records.get(nb_records - 1).unwrap(),
                    socket_records.get(nb_records - 2).unwrap(),
                ))
                .unwrap();
                power = power_record.value.clone();
                unit = power_record.unit.to_string();
            }
            let mut rec_j_1 = String::from("unknown");
            let mut rec_j_2 = String::from("unknown");
            let mut rec_j_3 = String::from("unknown");
            if socket_records.len() > 2 {
                rec_j_1 = socket_records
                    .get(nb_records - 3)
                    .unwrap()
                    .value
                    .to_string()
                    .trim()
                    .to_string();
            }
            if socket_records.len() > 1 {
                rec_j_2 = socket_records
                    .get(nb_records - 2)
                    .unwrap()
                    .value
                    .to_string()
                    .trim()
                    .to_string();
            }
            if !socket_records.is_empty() {
                rec_j_3 = socket_records
                    .get(nb_records - 1)
                    .unwrap()
                    .value
                    .to_string()
                    .trim()
                    .to_string();
            }
            let (socket_stats_user, socket_stats_nice, socket_stats_system) =
                match socket.get_stats_diff() {
                    Some(stat) => (
                        stat.cputime.user.to_string(),
                        stat.cputime.nice.to_string(),
                        stat.cputime.system.to_string(),
                    ),
                    None => (
                        String::from("n/a"),
                        String::from("n/a"),
                        String::from("n/a"),
                    ),
                };
            println!(
                "socket:{} {} {} last3(uJ): {} {} {} user {} nice {} system {}",
                socket_id,
                power,
                unit,
                rec_j_1,
                rec_j_2,
                rec_j_3,
                socket_stats_user,
                socket_stats_nice,
                socket_stats_system
            );

            for domain in socket.get_domains() {
                let domain_records = domain.get_records_passive();
                let mut power = String::from("unknown");
                let mut unit = String::from("W");
                let nb_records = domain_records.len();
                if nb_records > 1 {
                    let power_record = &energy_records_to_power_record((
                        domain_records.get(nb_records - 1).unwrap(),
                        domain_records.get(nb_records - 2).unwrap(),
                    ))
                    .unwrap();
                    power = power_record.value.clone();
                    unit = power_record.unit.to_string();
                }
                let mut rec_dom_j_1 = String::from("unknown");
                let mut rec_dom_j_2 = String::from("unknown");
                let mut rec_dom_j_3 = String::from("unknown");
                if domain_records.len() > 2 {
                    rec_dom_j_1 = domain_records
                        .get(nb_records - 3)
                        .unwrap()
                        .value
                        .to_string()
                        .trim()
                        .to_string();
                }
                if domain_records.len() > 1 {
                    rec_dom_j_2 = domain_records
                        .get(nb_records - 2)
                        .unwrap()
                        .value
                        .to_string()
                        .trim()
                        .to_string();
                }
                if !domain_records.is_empty() {
                    rec_dom_j_3 = domain_records
                        .get(nb_records - 1)
                        .unwrap()
                        .value
                        .to_string()
                        .trim()
                        .to_string();
                }
                println!(
                    "socket:{} domain:{}:{} {} {} last3(uJ): {} {} {}",
                    socket_id,
                    domain.id,
                    domain.name.trim(),
                    power,
                    unit,
                    rec_dom_j_1,
                    rec_dom_j_2,
                    rec_dom_j_3
                );
            }
        }
        println!(
            "topo stats: power: {}W user {} nice {} system {}",
            topo_power, topo_stats_user, topo_stats_nice, topo_stats_system
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn get_cons_socket0() {}
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
