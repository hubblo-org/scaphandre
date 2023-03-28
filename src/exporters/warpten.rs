use super::utils::get_hostname;
use crate::exporters::*;
use crate::sensors::{Sensor, Topology};
use clap::{value_parser, Arg};
use std::time::Duration;
use std::{env, thread};

//use warp10::data::Format;

/// An exporter that sends power consumption data of the host and its processes to
/// a [Warp10](https://warp10.io) instance through **HTTP(s)**
/// (contributions welcome to support websockets).
pub struct Warp10Exporter {
    sensor: Box<dyn Sensor>,
}

impl Exporter for Warp10Exporter {
    /// Control loop for self.iteration()
    fn run(&mut self, parameters: clap::ArgMatches) {
        let host = parameters.get_one::<String>("host").unwrap();
        let scheme = parameters.get_one::<String>("scheme").unwrap();
        let port = parameters.get_one::<String>("port").unwrap();
        let write_token = if let Some(token) = parameters.get_one::<String>("write-token") {
            token.to_owned()
        } else {
            match env::var("SCAPH_WARP10_WRITE_TOKEN") {
                Ok(val) => val,
                Err(_e) => panic!(
                    "SCAPH_WARP10_WRITE_TOKEN not found in env, nor write-token flag was used."
                ),
            }
        };
        //let read_token = parameters.value_of("read-token");
        let step: u64 = *parameters.get_one("step").unwrap();
        let qemu = parameters.get_flag("qemu");
        let watch_containers = parameters.get_flag("containers");

        loop {
            match self.iteration(
                host,
                scheme,
                port.parse::<u16>().unwrap(),
                &write_token,
                qemu,
                watch_containers,
            ) {
                Ok(res) => debug!("Result: {:?}", res),
                Err(err) => error!("Failed ! {:?}", err),
            }
            thread::sleep(Duration::new(step, 0));
        }
    }

    /// Options for configuring the exporter.
    fn get_options() -> Vec<clap::Arg> {
        let mut options = Vec::new();
        let arg = Arg::new("host")
            .default_value("localhost")
            .help("Warp10 host's FQDN or IP address to send data to")
            .long("host")
            .short('H')
            .required(false)
            .action(clap::ArgAction::Set);
        options.push(arg);

        let arg = Arg::new("scheme")
            .default_value("http")
            .help("Either 'http' or 'https'")
            .long("scheme")
            .short('s')
            .required(false)
            .action(clap::ArgAction::Set);
        options.push(arg);

        let arg = Arg::new("port")
            .default_value("8080")
            .help("TCP port to join Warp10 on the host")
            .long("port")
            .short('p')
            .required(false)
            .action(clap::ArgAction::Set);
        options.push(arg);

        let arg = Arg::new("write-token")
            .help("Auth. token to write on Warp10")
            .long("write-token")
            .short('t')
            .required(false)
            .action(clap::ArgAction::Set);
        options.push(arg);

        let arg = Arg::new("step")
            .default_value("30")
            .help("Time step between measurements, in seconds.")
            .long("step")
            .short('S')
            .required(false)
            .value_parser(value_parser!(u64))
            .action(clap::ArgAction::Set);
        options.push(arg);

        let arg = Arg::new("qemu")
            .help("Tells scaphandre it is running on a Qemu hypervisor.")
            .long("qemu")
            .short('q')
            .required(false)
            .action(clap::ArgAction::SetTrue);
        options.push(arg);

        let arg = Arg::new("containers")
            .help("Monitor and apply labels for processes running as containers")
            .long("containers")
            .required(false)
            .action(clap::ArgAction::SetTrue);
        options.push(arg);

        options
    }
}

impl Warp10Exporter {
    /// Instantiates and returns a new Warp10Exporter
    pub fn new(sensor: Box<dyn Sensor>) -> Warp10Exporter {
        Warp10Exporter { sensor }
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
        watch_containers: bool,
    ) -> Result<Vec<warp10::Warp10Response>, warp10::Error> {
        let client = warp10::Client::new(&format!("{scheme}://{host}:{port}"))?;
        let writer = client.get_writer(write_token.to_string());

        let topology: Topology;

        match *self.sensor.get_topology() {
            Some(topo) => {
                topology = topo;
            }
            None => {
                panic!("Couldn't generate the Topology");
            }
        }

        let mut metric_generator =
            MetricGenerator::new(topology, get_hostname(), qemu, watch_containers);
        metric_generator
            .topology
            .proc_tracker
            .clean_terminated_process_records_vectors();

        debug!("Refreshing topology.");
        metric_generator.topology.refresh();

        metric_generator.gen_all_metrics();

        let mut process_data: Vec<warp10::Data> = vec![];

        for metric in metric_generator.pop_metrics() {
            let mut labels = vec![];

            for (k, v) in metric.attributes {
                labels.push(warp10::Label::new(&k, &v));
            }

            process_data.push(warp10::Data::new(
                time::OffsetDateTime::now_utc(),
                None,
                metric.name,
                labels,
                warp10::Value::String(metric.metric_value.to_string()),
            ));
        }

        let res = writer.post_sync(process_data)?;

        let results = vec![res];

        //let mut process_data = vec![warp10::Data::new(
        //    time::OffsetDateTime::now_utc(),
        //    None,
        //    String::from("scaph_self_version"),
        //    labels.clone(),
        //    warp10::Value::Double(scaphandre_version.parse::<f64>().unwrap()),
        //)];

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
