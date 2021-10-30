use crate::exporters::*;
use crate::sensors::{RecordGenerator, Sensor, Topology};
use clap::Arg;
use std::time::Duration;
use std::{env, thread};
use utils::get_scaphandre_version;
//use warp10::data::Format;

/// An exporter that sends power consumption data of the host and its processes to
/// a [Warp10](https://warp10.io) instance through **HTTP(s)**
/// (contributions welcome to support websockets).
pub struct Warp10Exporter {
    topology: Topology,
}

impl Exporter for Warp10Exporter {
    /// Control loop for self.iteration()
    fn run(&mut self, parameters: clap::ArgMatches) {
        let host = parameters.value_of("host").unwrap();
        let scheme = parameters.value_of("scheme").unwrap();
        let port = parameters.value_of("port").unwrap();
        let write_token;
        if let Some(token) = parameters.value_of("write-token") {
            write_token = token.to_owned();
        } else {
            write_token = match env::var("SCAPH_WARP10_WRITE_TOKEN") {
                Ok(val) => val,
                Err(_e) => panic!(
                    "SCAPH_WARP10_WRITE_TOKEN not found in env, nor write-token flag was used."
                ),
            };
        }
        //let read_token = parameters.value_of("read-token");
        let step = parameters.value_of("step").unwrap();
        let qemu = parameters.is_present("qemu");

        loop {
            match self.iteration(
                host,
                scheme,
                port.parse::<u16>().unwrap(),
                &write_token,
                //read_token,
                qemu,
            ) {
                Ok(res) => debug!("Result: {:?}", res),
                Err(err) => error!("Failed ! {:?}", err),
            }
            thread::sleep(Duration::new(step.parse::<u64>().unwrap(), 0));
        }
    }

    /// Options for configuring the exporter.
    fn get_options() -> Vec<clap::Arg<'static, 'static>> {
        let mut options = Vec::new();
        let arg = Arg::with_name("host")
            .default_value("localhost")
            .help("Warp10 host's FQDN or IP address to send data to")
            .long("host")
            .short("H")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("scheme")
            .default_value("http")
            .help("Either 'http' or 'https'")
            .long("scheme")
            .short("s")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("port")
            .default_value("8080")
            .help("TCP port to join Warp10 on the host")
            .long("port")
            .short("p")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("write-token")
            .help("Auth. token to write on Warp10")
            .long("write-token")
            .short("t")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("step")
            .default_value("30")
            .help("Time step between measurements, in seconds.")
            .long("step")
            .short("S")
            .required(false)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("qemu")
            .help("Tells scaphandre it is running on a Qemu hypervisor.")
            .long("qemu")
            .short("q")
            .required(false)
            .takes_value(false);
        options.push(arg);

        options
    }
}

impl Warp10Exporter {
    /// Instantiates and returns a new Warp10Exporter
    pub fn new(mut sensor: Box<dyn Sensor>) -> Warp10Exporter {
        if let Some(topo) = *sensor.get_topology() {
            Warp10Exporter { topology: topo }
        } else {
            error!("Could'nt generate the Topology.");
            panic!("Could'nt generate the Topology.");
        }
    }

    /// Collects data from the Topology, creates warp10::Data objects containing the
    /// metric itself and some labels attaches, stores them in a vector and sends it
    /// to Warp10
    pub fn iteration(
        &mut self,
        host: &str,
        scheme: &str,
        port: u16,
        write_token: &str,
        //read_token: Option<&str>,
        qemu: bool,
    ) -> Result<Vec<warp10::Warp10Response>, warp10::Error> {
        let client = warp10::Client::new(&format!("{}://{}:{}", scheme, host, port))?;
        let writer = client.get_writer(write_token.to_string());
        self.topology
            .proc_tracker
            .clean_terminated_process_records_vectors();

        debug!("Refreshing topology.");
        self.topology.refresh();

        let records = self.topology.get_records_passive();
        let scaphandre_version = get_scaphandre_version();

        let labels = vec![];

        let mut data = vec![warp10::Data::new(
            time::OffsetDateTime::now_utc(),
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
                time::OffsetDateTime::now_utc(),
                None,
                String::from("scaph_self_cpu_usage_percent"),
                labels.clone(),
                warp10::Value::Int(metric_value.value.parse::<i32>().unwrap()),
            ));
        }

        if let Some(metric_value) = self
            .topology
            .get_process_cpu_consumption_percentage(procfs::process::Process::myself().unwrap().pid)
        {
            data.push(warp10::Data::new(
                time::OffsetDateTime::now_utc(),
                None,
                String::from("scaph_self_cpu_usage_percent"),
                labels.clone(),
                warp10::Value::Int(metric_value.value.parse::<i32>().unwrap()),
            ));
        }

        if let Ok(metric_value) = procfs::process::Process::myself().unwrap().statm() {
            let value = metric_value.size * procfs::page_size().unwrap() as u64;
            data.push(warp10::Data::new(
                time::OffsetDateTime::now_utc(),
                None,
                String::from("scaph_self_mem_total_program_size"),
                labels.clone(),
                warp10::Value::Int(value as i32),
            ));
            let value = metric_value.resident * procfs::page_size().unwrap() as u64;
            data.push(warp10::Data::new(
                time::OffsetDateTime::now_utc(),
                None,
                String::from("scaph_self_mem_resident_set_size"),
                labels.clone(),
                warp10::Value::Int(value as i32),
            ));
            let value = metric_value.shared * procfs::page_size().unwrap() as u64;
            data.push(warp10::Data::new(
                time::OffsetDateTime::now_utc(),
                None,
                String::from("scaph_self_mem_shared_resident_size"),
                labels.clone(),
                warp10::Value::Int(value as i32),
            ));
        }

        let metric_value = self.topology.stat_buffer.len();
        data.push(warp10::Data::new(
            time::OffsetDateTime::now_utc(),
            None,
            String::from("scaph_self_topo_stats_nb"),
            labels.clone(),
            warp10::Value::Int(metric_value as i32),
        ));

        let metric_value = self.topology.record_buffer.len();
        data.push(warp10::Data::new(
            time::OffsetDateTime::now_utc(),
            None,
            String::from("scaph_self_topo_records_nb"),
            labels.clone(),
            warp10::Value::Int(metric_value as i32),
        ));

        let metric_value = self.topology.proc_tracker.procs.len();
        data.push(warp10::Data::new(
            time::OffsetDateTime::now_utc(),
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
                time::OffsetDateTime::now_utc(),
                None,
                String::from("scaph_self_socket_stats_nb"),
                metric_labels.clone(),
                warp10::Value::Int(metric_value as i32),
            ));
            let metric_value = socket.record_buffer.len();
            data.push(warp10::Data::new(
                time::OffsetDateTime::now_utc(),
                None,
                String::from("scaph_self_socket_records_nb"),
                metric_labels.clone(),
                warp10::Value::Int(metric_value as i32),
            ));

            let socket_records = socket.get_records_passive();
            if !socket_records.is_empty() {
                let socket_energy_microjoules = &socket_records.last().unwrap().value;
                if let Ok(metric_value) = socket_energy_microjoules.parse::<i64>() {
                    data.push(warp10::Data::new(
                        time::OffsetDateTime::now_utc(),
                        None,
                        String::from("scaph_socket_energy_microjoules"),
                        metric_labels.clone(),
                        warp10::Value::Long(metric_value),
                    ));
                }

                if let Some(metric_value) = socket.get_records_diff_power_microwatts() {
                    data.push(warp10::Data::new(
                        time::OffsetDateTime::now_utc(),
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
                    time::OffsetDateTime::now_utc(),
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
                time::OffsetDateTime::now_utc(),
                None,
                String::from("scaph_host_energy_microjoules"),
                labels.clone(),
                warp10::Value::Long(metric_value.parse::<i64>().unwrap()),
            ));

            if let Some(metric_value) = self.topology.get_records_diff_power_microwatts() {
                data.push(warp10::Data::new(
                    time::OffsetDateTime::now_utc(),
                    None,
                    String::from("scaph_host_power_microwatts"),
                    labels.clone(),
                    warp10::Value::Long(metric_value.value.parse::<i64>().unwrap()),
                ));
            }
        }

        let res = writer.post_sync(data)?;

        let mut results = vec![res];

        let mut process_data = vec![warp10::Data::new(
            time::OffsetDateTime::now_utc(),
            None,
            String::from("scaph_self_version"),
            labels.clone(),
            warp10::Value::Double(scaphandre_version.parse::<f64>().unwrap()),
        )];

        let processes_tracker = &self.topology.proc_tracker;
        for pid in processes_tracker.get_alive_pids() {
            let exe = processes_tracker.get_process_name(pid);
            let cmdline = processes_tracker.get_process_cmdline(pid);

            let mut plabels = labels.clone();
            plabels.push(warp10::Label::new("pid", &pid.to_string()));
            plabels.push(warp10::Label::new("exe", &exe));
            if let Some(cmdline_str) = cmdline {
                if qemu {
                    if let Some(vmname) = utils::filter_qemu_cmdline(&cmdline_str) {
                        plabels.push(warp10::Label::new("vmname", &vmname));
                    }
                }
                plabels.push(warp10::Label::new(
                    "cmdline",
                    &cmdline_str.replace("\"", "\\\""),
                ));
            }
            let metric_name = format!(
                "{}_{}_{}",
                "scaph_process_power_consumption_microwats",
                pid.to_string(),
                exe
            );
            if let Some(power) = self.topology.get_process_power_consumption_microwatts(pid) {
                process_data.push(warp10::Data::new(
                    time::OffsetDateTime::now_utc(),
                    None,
                    metric_name,
                    plabels,
                    warp10::Value::Long(power.value.parse::<i64>().unwrap()),
                ));
            }
        }
        let process_res = writer.post_sync(process_data)?;

        //if let Some(token) = read_token {
        //let reader = client.get_reader(token.to_owned());
        //let parameters = warp10::data::ParameterSet::new(
        //"scaph_host_power_microwatts{}".to_string(),
        //Format::Text,
        //None, None, None,
        //Some(String::from("now")), Some(String::from("-10")),
        //None, None, None
        //);
        //let response = reader.get_sync(parameters);
        //match response {
        //Ok(resp) => warn!("response is: {:?}", resp),
        //Err(err) => panic!("error is: {:?}", err)
        //}
        //}

        results.push(process_res);

        Ok(results)
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
