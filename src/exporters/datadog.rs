use crate::exporters::*;
use crate::sensors::{Sensor, Topology};
use clap::Arg;
use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub enum Type {
    Count,
    Gauge,
    Rate,
}

impl Type {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Count => "count",
            Self::Gauge => "gauge",
            Self::Rate => "rate",
        }
    }
}

impl Serialize for Type {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[derive(Clone, Debug)]
pub struct Point {
    timestamp: u64,
    value: f64,
}

impl Point {
    pub fn new(timestamp: u64, value: f64) -> Self {
        Self { timestamp, value }
    }
}

impl Serialize for Point {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&self.timestamp)?;
        seq.serialize_element(&self.value)?;
        seq.end()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Serie {
    // The name of the host that produced the metric.
    #[serde(skip_serializing_if = "Option::is_none")]
    host: Option<String>,
    // If the type of the metric is rate or count, define the corresponding interval.
    #[serde(skip_serializing_if = "Option::is_none")]
    interval: Option<i64>,
    // The name of the timeseries.
    metric: String,
    // Points relating to a metric. All points must be tuples with timestamp and a scalar value (cannot be a string).
    // Timestamps should be in POSIX time in seconds, and cannot be more than ten minutes in the future or more than one hour in the past.
    points: Vec<Point>,
    // A list of tags associated with the metric.
    tags: Vec<String>,
    // The type of the metric either count, gauge, or rate.
    #[serde(rename = "type")]
    dtype: Type,
}

impl Serie {
    pub fn new(metric: &str, dtype: Type) -> Self {
        Self {
            host: None,
            interval: None,
            metric: metric.to_string(),
            points: Vec::new(),
            tags: Vec::new(),
            dtype,
        }
    }
}

impl Serie {
    pub fn set_host(mut self, host: &str) -> Self {
        self.host = Some(host.to_string());
        self
    }

    pub fn set_interval(mut self, interval: i64) -> Self {
        self.interval = Some(interval);
        self
    }

    pub fn set_points(mut self, points: Vec<Point>) -> Self {
        self.points = points;
        self
    }

    pub fn add_point(mut self, point: Point) -> Self {
        self.points.push(point);
        self
    }
}

impl Serie {
    pub fn set_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn add_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }
}

struct Client {
    host: String,
    api_key: String,
}

impl Client {
    pub fn new(parameters: &ArgMatches) -> Self {
        Self {
            host: parameters.value_of("host").unwrap().to_string(),
            api_key: parameters.value_of("api_key").unwrap().to_string(),
        }
    }

    pub fn send(&self, series: &[Serie]) {
        let url = format!("{}/api/v1/series", self.host);
        let request = ureq::post(url.as_str())
            .set("DD-API-KEY", self.api_key.as_str())
            .send_json(serde_json::json!({ "series": series }));
        match request {
            Ok(response) => {
                if response.status() >= 400 {
                    log::warn!(
                        "couldn't send metrics to datadog: status {}",
                        response.status_text()
                    );
                    if let Ok(body) = response.into_string() {
                        log::warn!("response from server: {}", body);
                    }
                } else {
                    log::info!("metrics sent with success");
                }
            }
            Err(err) => log::warn!("error while sending metrics: {}", err),
        };
    }
}

fn merge<A>(first: Vec<A>, second: Vec<A>) -> Vec<A> {
    second.into_iter().fold(first, |mut res, item| {
        res.push(item);
        res
    })
}

fn get_domain_name(index: usize) -> Option<&'static str> {
    match index {
        0 => Some("core"),
        1 => Some("uncore"),
        2 => Some("dram"),
        _ => None,
    }
}

/// An Exporter that displays power consumption data of the host
/// and its processes on the standard output of the terminal.
pub struct DatadogExporter {
    topology: Topology,
    hostname: String,
}

impl Exporter for DatadogExporter {
    /// Lanches runner()
    fn run(&mut self, parameters: ArgMatches) {
        self.runner(&parameters);
    }

    /// Returns options needed for that exporter, as a HashMap
    fn get_options() -> Vec<clap::Arg<'static, 'static>> {
        let mut options = Vec::new();
        let arg = Arg::with_name("host")
            .default_value("https://api.datadoghq.eu")
            .help("The domain of the datadog instance.")
            .long("host")
            .short("h")
            .required(true)
            .takes_value(true);
        options.push(arg);

        let arg = Arg::with_name("api_key")
            .long("api_key")
            .short("k")
            .required(true)
            .takes_value(true)
            .help("Api key to authenticate with datadog.");
        options.push(arg);

        let arg = Arg::with_name("timeout")
            .long("timeout")
            .short("t")
            .required(false)
            .takes_value(true)
            .help("Maximum time to collect and ship the metrics.");
        options.push(arg);

        let arg = Arg::with_name("step_duration")
            .long("step-duration")
            .default_value("20")
            .short("s")
            .required(false)
            .takes_value(true)
            .help("Time step duration between two measurements, in seconds.");
        options.push(arg);

        let arg = Arg::with_name("step_duration_nano")
            .long("step-duration-nano")
            .default_value("0")
            .short("n")
            .required(false)
            .takes_value(true)
            .help("Time step duration between two measurments, in nano seconds. This is cumulative to step-duration.");
        options.push(arg);

        options
    }
}

impl DatadogExporter {
    /// Instantiates and returns a new DatadogExporter
    pub fn new(mut sensor: Box<dyn Sensor>) -> DatadogExporter {
        let some_topology = *sensor.get_topology();

        DatadogExporter {
            topology: some_topology.unwrap(),
            hostname: hostname::get()
                .expect("unable to get hostname")
                .to_str()
                .unwrap()
                .to_string(),
        }
    }

    fn runner(&mut self, parameters: &ArgMatches<'_>) {
        let client = Client::new(parameters);
        warn!("runner");
        // We have a default value of 2s so it is safe to unwrap the option
        // Panic if a non numerical value is passed
        let step_duration: u64 = parameters
            .value_of("step_duration")
            .unwrap()
            .parse::<u64>()
            .expect("Wrong step_duration value, should be a number of seconds");
        let step_duration_nano: u32 = parameters
            .value_of("step_duration_nano")
            .unwrap()
            .parse::<u32>()
            .expect("Wrong step_duration_nano value, should be a number of nano seconds");

        info!(
            "Measurement step is: {}s{}ns",
            step_duration, step_duration_nano
        );
        if let Some(timeout) = parameters.value_of("timeout") {
            let now = Instant::now();
            let timeout = timeout
                .parse::<u64>()
                .expect("Wrong timeout value, should be a number of seconds");

            while now.elapsed().as_secs() <= timeout {
                warn!("iterate");
                self.iterate(&client);
                thread::sleep(Duration::new(step_duration, step_duration_nano));
            }
        } else {
            loop {
                self.iterate(&client);
                thread::sleep(Duration::new(step_duration, step_duration_nano));
            }
        }
    }

    fn iterate(&mut self, client: &Client) {
        self.topology.refresh();
        let series = self.collect_series();
        client.send(&series);
    }

    fn create_consumption_serie(&self) -> Serie {
        Serie::new("consumption", Type::Gauge)
            .set_host(self.hostname.as_str())
            .add_tag(format!("hostname:{}", self.hostname))
    }

    fn collect_process_series(&mut self) -> Vec<Serie> {
        let record = match self.topology.get_records_diff_power_microwatts() {
            Some(item) => item,
            None => return vec![],
        };
        let host_stat = match self.topology.get_stats_diff() {
            Some(item) => item,
            None => return vec![],
        };
        let host_power_ts = record.timestamp.as_secs();
        let host_power = record.value.parse::<u64>().unwrap_or(0) as f32;

        let ticks_per_second = procfs::ticks_per_second().unwrap() as f32;

        let consumers = self.topology.proc_tracker.get_top_consumers(10);
        consumers
            .iter()
            .map(|item| {
                let host_time = host_stat.total_time_jiffies();
                let consumption = (item.1 as f32 / (host_time * ticks_per_second)) * host_power;
                let exe = item
                    .0
                    .exe()
                    .ok()
                    .and_then(|v| v.to_str().map(|s| s.to_string()))
                    .unwrap_or_default();
                let point = Point::new(host_power_ts, consumption as f64);
                self.create_consumption_serie()
                    .add_point(point)
                    .add_tag(format!("process.exe:{}", exe))
                    .add_tag(format!("process.pid:{}", item.0.pid()))
            })
            .collect::<Vec<_>>()
    }

    fn collect_socket_series(&mut self) -> Vec<Serie> {
        self.topology
            .get_sockets_passive()
            .iter()
            .fold(Vec::new(), |mut res, socket| {
                let socket_record = match socket.get_records_diff_power_microwatts() {
                    Some(item) => item,
                    None => return res,
                };
                let socket_power = socket_record.value.parse::<u64>().unwrap_or(0);
                res.push(
                    self.create_consumption_serie()
                        .add_point(Point::new(
                            socket_record.timestamp.as_secs(),
                            socket_power as f64,
                        ))
                        .add_tag(format!("socket.id:{}", socket.id)),
                );
                socket
                    .get_domains_passive()
                    .iter()
                    .map(|d| d.get_records_diff_power_microwatts())
                    .enumerate()
                    .filter_map(|(index, record)| {
                        let name = match get_domain_name(index) {
                            Some(name) => name,
                            None => return None,
                        };
                        let record = match record {
                            Some(item) => item,
                            None => return None,
                        };
                        Some((
                            name,
                            Point::new(
                                record.timestamp.as_secs(),
                                record.value.parse::<u64>().unwrap_or(0) as f64,
                            ),
                        ))
                    })
                    .fold(res, |mut res, (name, point)| {
                        res.push(
                            self.create_consumption_serie()
                                .add_point(point)
                                .add_tag(format!("socket.id:{}", socket.id))
                                .add_tag(format!("socket.domain:{}", name)),
                        );
                        res
                    })
            })
    }

    fn collect_series(&mut self) -> Vec<Serie> {
        let processes = self.collect_process_series();
        let sockets = self.collect_socket_series();
        merge(processes, sockets)
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
