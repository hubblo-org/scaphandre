use crate::exporters::*;
use crate::sensors::{RecordGenerator, Sensor, Topology};
use std::collections::HashMap;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use utils::get_scaphandre_version;
use warp10;

pub struct Warp10Exporter {
    topology: Topology,
}

impl Exporter for Warp10Exporter {
    //info!("Starting Warp10 exporter");
    fn run(&mut self, parameters: clap::ArgMatches) {
        let host = parameters.value_of("host").unwrap();
        let scheme = parameters.value_of("scheme").unwrap();
        let port = parameters.value_of("port").unwrap();
        let write_token = parameters.value_of("write-token").unwrap();
        let step = parameters.value_of("step").unwrap();

        loop {
            match self.iteration(host, scheme, port.parse::<u16>().unwrap(), write_token) {
                Ok(res) => println!("Result: {:?}", res),
                Err(err) => error!("Failed ! {:?}", err),
            }
            thread::sleep(Duration::new(step.parse::<u64>().unwrap(), 0));
        }
    }

    fn get_options() -> HashMap<String, super::ExporterOption> {
        let mut options = HashMap::new();

        options.insert(
            String::from("host"),
            ExporterOption {
                default_value: Some(String::from("localhost")),
                help: String::from("Warp10 host's FQDN or IP address to send data to"),
                long: String::from("host"),
                short: String::from("H"),
                required: false,
                takes_value: true,
            },
        );
        options.insert(
            String::from("scheme"),
            ExporterOption {
                default_value: Some(String::from("https")),
                help: String::from("Either 'http' or 'https'"),
                long: String::from("scheme"),
                short: String::from("s"),
                required: false,
                takes_value: true,
            },
        );
        options.insert(
            String::from("port"),
            ExporterOption {
                default_value: Some(String::from("8080")),
                help: String::from("TCP port to join Warp10 on the host"),
                long: String::from("port"),
                short: String::from("p"),
                required: false,
                takes_value: true,
            },
        );
        options.insert(
            String::from("write-token"),
            ExporterOption {
                default_value: None,
                help: String::from("Auth. token to write on Warp10"),
                long: String::from("write-token"),
                short: String::from("t"),
                required: true,
                takes_value: true,
            },
        );
        options.insert(
            String::from("step"),
            ExporterOption {
                default_value: Some(String::from("60")),
                help: String::from("Time step between measurements, in seconds."),
                long: String::from("step"),
                short: String::from("S"),
                required: true,
                takes_value: true,
            },
        );

        options
    }
}

impl Warp10Exporter {
    pub fn new(mut sensor: Box<dyn Sensor>) -> Warp10Exporter {
        if let Some(topo) = *sensor.get_topology() {
            return Warp10Exporter { topology: topo };
        } else {
            error!("Could'nt generate the Topology.");
            panic!("Could'nt generate the Topology.");
        }
    }

    pub fn iteration(
        &mut self,
        host: &str,
        scheme: &str,
        port: u16,
        write_token: &str,
    ) -> Result<warp10::Response, warp10::Error> {
        let client = warp10::Client::new(&format!("{}://{}:{}/", scheme, host, port))?;
        let writer = client.get_writer(write_token.to_string());
        self.topology
            .proc_tracker
            .clean_terminated_process_records_vectors();

        debug!("Refreshing topology.");
        self.topology.refresh();

        let records = self.topology.get_records_passive();
        let scaphandre_version = get_scaphandre_version();

        let labels = vec![
            warp10::Label::new("scaphandre_self_version", &scaphandre_version),
            warp10::Label::new("agent", "scaphandre"),
        ];

        let mut data = vec![warp10::Data::new(
            time::now_utc().to_timespec(),
            None,
            String::from("scaph_self_version"),
            labels.clone(),
            warp10::Value::Double(scaphandre_version.parse::<f64>().unwrap()),
        )];
        if let Some(metric_value) = self
            .topology
            .get_process_cpu_consumption_percentage(procfs::process::Process::myself().unwrap().pid)
        {
            data.push(warp10::Data::new(
                time::now_utc().to_timespec(),
                None,
                String::from("scaph_self_cpu_usage_percent"),
                labels.clone(),
                warp10::Value::Int(metric_value as i32),
            ));
        } else {
            error!("Failed to get self cpu percentage.");
        }

        if let Some(metric_value) = self
            .topology
            .get_process_cpu_consumption_percentage(procfs::process::Process::myself().unwrap().pid)
        {
            data.push(warp10::Data::new(
                time::now_utc().to_timespec(),
                None,
                String::from("scaph_self_cpu_usage_percent"),
                labels.clone(),
                warp10::Value::Int(metric_value as i32),
            ));
        }

        if let Ok(metric_value) = procfs::process::Process::myself().unwrap().statm() {
            let value = metric_value.size * procfs::page_size().unwrap() as u64;
            data.push(warp10::Data::new(
                time::now_utc().to_timespec(),
                None,
                String::from("scaph_self_mem_total_program_size"),
                labels.clone(),
                warp10::Value::Int(value as i32),
            ));
            let value = metric_value.resident * procfs::page_size().unwrap() as u64;
            data.push(warp10::Data::new(
                time::now_utc().to_timespec(),
                None,
                String::from("scaph_self_mem_resident_set_size"),
                labels.clone(),
                warp10::Value::Int(value as i32),
            ));
            let value = metric_value.shared * procfs::page_size().unwrap() as u64;
            data.push(warp10::Data::new(
                time::now_utc().to_timespec(),
                None,
                String::from("scaph_self_mem_shared_resident_size"),
                labels.clone(),
                warp10::Value::Int(value as i32),
            ));
        }

        let metric_value = self.topology.stat_buffer.len();
        data.push(warp10::Data::new(
            time::now_utc().to_timespec(),
            None,
            String::from("scaph_self_topo_stats_nb"),
            labels.clone(),
            warp10::Value::Int(metric_value as i32),
        ));

        let metric_value = self.topology.record_buffer.len();
        data.push(warp10::Data::new(
            time::now_utc().to_timespec(),
            None,
            String::from("scaph_self_topo_records_nb"),
            labels.clone(),
            warp10::Value::Int(metric_value as i32),
        ));

        let metric_value = self.topology.proc_tracker.procs.len();
        data.push(warp10::Data::new(
            time::now_utc().to_timespec(),
            None,
            String::from("scaph_self_topo_procs_nb"),
            labels.clone(),
            warp10::Value::Int(metric_value as i32),
        ));

        for socket in &self.topology.sockets {
            let mut metric_labels = labels.clone();
            metric_labels.push(warp10::Label::new("socket_id", &socket.id.to_string()));
            let metric_value = socket.stat_buffer.len();
            data.push(warp10::Data::new(
                time::now_utc().to_timespec(),
                None,
                String::from("scaph_self_socket_stats_nb"),
                metric_labels.clone(),
                warp10::Value::Int(metric_value as i32),
            ));
            let metric_value = socket.record_buffer.len();
            data.push(warp10::Data::new(
                time::now_utc().to_timespec(),
                None,
                String::from("scaph_self_socket_records_nb"),
                metric_labels.clone(),
                warp10::Value::Int(metric_value as i32),
            ));

            let socket_records = socket.get_records_passive();
            if !socket_records.is_empty() {
                let metric_value = &socket_records.last().unwrap().value;

                data.push(warp10::Data::new(
                    time::now_utc().to_timespec(),
                    None,
                    String::from("scaph_socket_energy_microjoules"),
                    metric_labels.clone(),
                    warp10::Value::Long(metric_value.parse::<i64>().unwrap()),
                ));

                if let Some(metric_value) = socket.get_records_diff_power_microwatts() {
                    data.push(warp10::Data::new(
                        time::now_utc().to_timespec(),
                        None,
                        String::from("scaph_socket_power_microwatts"),
                        metric_labels.clone(),
                        warp10::Value::Long(metric_value.value.parse::<i64>().unwrap()),
                    ));
                }
            }

            for domain in &socket.domains {
                let mut metric_labels = labels.clone();
                metric_labels.push(warp10::Label::new("rapl_domain_name", &domain.name));
                let metric_value = domain.record_buffer.len();
                data.push(warp10::Data::new(
                    time::now_utc().to_timespec(),
                    None,
                    String::from("scaph_self_domain_records_nb"),
                    metric_labels.clone(),
                    warp10::Value::Int(metric_value as i32),
                ));
            }
        }

        if !records.is_empty() {
            let record = records.last().unwrap();
            let metric_value = record.value.clone();

            data.push(warp10::Data::new(
                time::now_utc().to_timespec(),
                None,
                String::from("scaph_host_energy_microjoules"),
                labels.clone(),
                warp10::Value::Long(metric_value.parse::<i64>().unwrap()),
            ));

            if let Some(metric_value) = self.topology.get_records_diff_power_microwatts() {
                data.push(warp10::Data::new(
                    time::now_utc().to_timespec(),
                    None,
                    String::from("scaph_host_power_microwatts"),
                    labels.clone(),
                    warp10::Value::Long(metric_value.value.parse::<i64>().unwrap()),
                ));
            }
        }

        let res = writer.post(data)?;
        Ok(res)
    }
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
