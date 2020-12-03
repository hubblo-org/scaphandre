pub mod prometheus;
pub mod qemu;
pub mod stdout;
use clap::ArgMatches;
use std::collections::HashMap;

/// An Exporter is what tells scaphandre when to collect metrics and how to export
/// or expose them.
/// Its basic role is to instanciate a Sensor, get the data the sensor has to offer
/// and expose the data in the desired way. An exporter could either push the metrics
/// over the network to a remote destination, store those metrics on the filesystem
/// or expose them to be collected by another software. It decides at what pace
/// the metrics are generated/refreshed by calling the refresh* methods available
/// with the structs provided by the sensor.
pub trait Exporter {
    fn run(&mut self, parameters: ArgMatches);
    fn get_options() -> HashMap<String, ExporterOption>;
}

pub struct ExporterOption {
    /// States whether the option is mandatory or not
    pub required: bool,
    /// Does the option need a value to be specified ?
    pub takes_value: bool,
    /// The default value, if needed
    pub default_value: String,
    /// One letter to identify the option (useful for the CLI)
    pub short: String,
    /// A word to identify the option
    pub long: String,
    /// A brief description to explain what the option does
    pub help: String,
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
