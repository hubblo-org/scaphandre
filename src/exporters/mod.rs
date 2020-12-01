pub mod prometheus;
pub mod qemu;
pub mod stdout;
use clap::ArgMatches;
use std::collections::HashMap;

pub trait Exporter {
    fn run(&mut self, parameters: ArgMatches);
    fn get_options() -> HashMap<String, ExporterOption>;
}

pub struct ExporterOption {
    pub required: bool,
    pub takes_value: bool,
    pub default_value: String,
    pub short: String,
    pub long: String,
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
