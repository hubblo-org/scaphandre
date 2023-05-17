use super::utils::get_hostname;
use crate::exporters::*;
use crate::sensors::Sensor;
use std::time::Duration;

/// An exporter that sends power consumption data of the host and its processes to
/// a [Warp10](https://warp10.io) instance through **HTTP(s)**
/// (contributions welcome to support websockets).
pub struct Warp10Exporter {
    metric_generator: MetricGenerator,
    /// Warp10 client
    client: warp10::Client,
    /// Warp10 auth token
    write_token: String,

    step: Duration,
}

/// Holds the arguments for a Warp10Exporter.
#[derive(clap::Args, Debug)]
pub struct ExporterArgs {
    /// FQDN or IP address of the Warp10 instance
    #[arg(short = 'H', long, default_value = "localhost")]
    pub host: String,

    /// TCP port of the Warp10 instance
    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,

    /// "http" or "https"
    #[arg(short = 'S', long, default_value = "http")]
    pub scheme: String,

    /// Auth token to write data to Warp10.
    /// If not specified, you must set the env variable SCAPH_WARP10_WRITE_TOKEN
    #[arg(short = 't', long)]
    pub write_token: Option<String>,

    /// Interval between two measurements, in seconds
    #[arg(short, long, value_name = "SECONDS", default_value_t = 2)]
    pub step: u64,

    /// Apply labels to metrics of processes looking like a Qemu/KVM virtual machine
    #[arg(short, long)]
    pub qemu: bool,
}

const TOKEN_ENV_VAR: &str = "SCAPH_WARP10_WRITE_TOKEN";

impl Exporter for Warp10Exporter {
    /// Control loop for self.iterate()
    fn run(&mut self) {
        loop {
            match self.iterate() {
                Ok(res) => debug!("Result: {:?}", res),
                Err(err) => error!("Failed ! {:?}", err),
            }
            std::thread::sleep(self.step);
        }
    }

    fn kind(&self) -> &str {
        "warp10"
    }
}

impl Warp10Exporter {
    /// Instantiates and returns a new Warp10Exporter
    pub fn new(sensor: &dyn Sensor, args: ExporterArgs) -> Warp10Exporter {
        // Prepare for measurement
        let topology = sensor
            .get_topology()
            .expect("sensor topology should be available");
        let metric_generator = MetricGenerator::new(topology, get_hostname(), args.qemu, false);

        // Prepare for sending data to Warp10
        let scheme = args.scheme;
        let host = args.host;
        let port = args.port;
        let client = warp10::Client::new(&format!("{scheme}://{host}:{port}"))
            .expect("warp10 Client could not be created");
        let write_token = args.write_token.unwrap_or_else(|| {
            std::env::var(TOKEN_ENV_VAR).unwrap_or_else(|_| panic!("No token found, you must provide either --write-token or the env var {TOKEN_ENV_VAR}"))
        });

        Warp10Exporter {
            metric_generator,
            client,
            write_token,
            step: Duration::from_secs(args.step),
        }
    }

    /// Collects data from the Topology, creates warp10::Data objects containing the
    /// metric itself and some labels attaches, stores them in a vector and sends it
    /// to Warp10
    pub fn iterate(&mut self) -> Result<Vec<warp10::Warp10Response>, warp10::Error> {
        let writer = self.client.get_writer(self.write_token.clone());
        self.metric_generator
            .topology
            .proc_tracker
            .clean_terminated_process_records_vectors();

        debug!("Refreshing topology.");
        self.metric_generator.topology.refresh();

        self.metric_generator.gen_all_metrics();

        let mut process_data: Vec<warp10::Data> = vec![];

        for metric in self.metric_generator.pop_metrics() {
            let mut labels = vec![];

            for (k, v) in &metric.attributes {
                labels.push(warp10::Label::new(k, v));
            }

            process_data.push(warp10::Data::new(
                time::OffsetDateTime::now_utc(),
                None,
                metric.name,
                labels,
                warp10::Value::String(metric.metric_value.to_string().replace('`', "")),
            ));
        }

        let res = writer.post_sync(process_data)?;

        let results = vec![res];

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
