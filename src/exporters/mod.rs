pub mod stdout;
pub mod prometheus;
use std::collections::HashMap;
use clap::ArgMatches;

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
    pub help: String
}