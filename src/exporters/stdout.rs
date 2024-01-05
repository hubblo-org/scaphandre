use crate::exporters::*;
use crate::sensors::{utils::current_system_time_since_epoch, utils::IProcess, Sensor};
use regex::Regex;
use std::fmt::Write;
use std::thread;
use std::time::{Duration, Instant};

/// An Exporter that displays power consumption data of the host
/// and its processes on the standard output of the terminal.
pub struct StdoutExporter {
    metric_generator: MetricGenerator,
    args: ExporterArgs,
}

/// Holds the arguments for a StdoutExporter.
///
/// When using Scaphandre as a command-line application, such a struct will be
/// automatically populated by the clap library. If you're using Scaphandre as
/// a library, you should populate the arguments yourself.
#[derive(clap::Args, Debug)]
// The command group makes `processes` and `regex_filter` exclusive.
#[command(group(clap::ArgGroup::new("disp").args(["processes", "regex_filter"])))]
pub struct ExporterArgs {
    /// Maximum time spent measuring, in seconds.
    /// If negative, runs forever.
    #[arg(short, long, default_value_t = 10)]
    pub timeout: i64,

    /// Interval between two measurements, in seconds
    #[arg(short, long, value_name = "SECONDS", default_value_t = 2)]
    pub step: u64,

    /// Maximum number of processes to display
    #[arg(short, long, default_value_t = 5)]
    pub processes: u16,

    /// Filter processes based on regular expressions (example: 'scaph\\w\\w.e')
    #[arg(short, long)]
    pub regex_filter: Option<Regex>,

    /// Monitor and apply labels for processes running as containers
    #[arg(long)]
    pub containers: bool,

    /// Apply labels to metrics of processes looking like a Qemu/KVM virtual machine
    #[arg(short, long)]
    pub qemu: bool,

    /// Display metrics with their names
    #[arg(long)]
    pub raw_metrics: bool,
}

impl Exporter for StdoutExporter {
    /// Runs [iterate()] every `step` until `timeout`
    fn run(&mut self) {
        let time_step = Duration::from_secs(self.args.step);
        let time_limit = if self.args.timeout < 0 {
            None
        } else {
            Some(Duration::from_secs(self.args.timeout.unsigned_abs()))
        };

        println!("Measurement step is: {time_step:?}");
        if let Some(timeout) = time_limit {
            let t0 = Instant::now();
            while t0.elapsed() <= timeout {
                self.iterate();
                thread::sleep(time_step);
            }
        } else {
            loop {
                self.iterate();
                thread::sleep(time_step);
            }
        }
    }

    fn kind(&self) -> &str {
        "stdout"
    }
}

impl StdoutExporter {
    /// Instantiates and returns a new StdoutExporter
    pub fn new(sensor: &dyn Sensor, args: ExporterArgs) -> StdoutExporter {
        // Prepare the retrieval of the measurements
        let topo = sensor
            .get_topology()
            .expect("sensor topology should be available");

        let metric_generator =
            MetricGenerator::new(topo, utils::get_hostname(), args.qemu, args.containers);

        StdoutExporter {
            metric_generator,
            args,
        }
    }

    fn iterate(&mut self) {
        self.metric_generator
            .topology
            .proc_tracker
            .clean_terminated_process_records_vectors();
        self.metric_generator.topology.refresh();
        self.show_metrics();
    }

    fn summarized_view(&mut self, metrics: Vec<Metric>) {
        let mut metrics_iter = metrics.iter();
        let none_value = MetricValueType::Text("0".to_string());
        let mut host_power_source = String::from("");
        let host_power = match metrics_iter.find(|x| x.name == "scaph_host_power_microwatts") {
            Some(m) => {
                if let Some(src) = &m.attributes.get("value_source") {
                    host_power_source = src.to_string()
                }
                &m.metric_value
            }
            None => &none_value,
        };

        let domain_names = self.metric_generator.topology.domains_names.as_ref();
        if domain_names.is_some() {
            info!("domain_names: {:?}", domain_names.unwrap());
        }

        println!(
            "Host:\t{} W from {}",
            (format!("{host_power}").parse::<f64>().unwrap() / 1000000.0),
            host_power_source
        );

        if domain_names.is_some() {
            println!("\tpackage \t{}", domain_names.unwrap().join("\t\t"));
        }

        for s in metrics
            .iter()
            .filter(|x| x.name == "scaph_socket_power_microwatts")
        {
            debug!("✅ Found socket power metric !");
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
            } else {
                println!("{to_print} Could'nt get per-domain metrics.\n");
            }
        }

        let consumers: Vec<(IProcess, f64)>;
        if let Some(regex) = &self.args.regex_filter {
            println!("Processes filtered by '{regex}':");
            consumers = self
                .metric_generator
                .topology
                .proc_tracker
                .get_filtered_processes(regex);
        } else {
            let n = self.args.processes;
            println!("Top {n} consumers:");
            consumers = self
                .metric_generator
                .topology
                .proc_tracker
                .get_top_consumers(n);
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
                        pid == &c.0.pid.to_string()
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

    fn raw_metrics_view(&mut self, metrics: Vec<Metric>) {
        println!("## At {}", current_system_time_since_epoch().as_secs());
        for m in metrics {
            let serialized_data = serde_json::to_string(&m.attributes).unwrap();
            println!(
                "{} = {} {} # {}",
                m.name, m.metric_value, serialized_data, m.description
            );
        }
    }

    fn show_metrics(&mut self) {
        self.metric_generator.gen_all_metrics();

        let metrics = self.metric_generator.pop_metrics();

        if self.args.raw_metrics {
            self.raw_metrics_view(metrics);
        } else {
            self.summarized_view(metrics);
        }
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
