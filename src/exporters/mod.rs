pub mod stdout;
pub mod prometheus;
use std::collections::HashMap;

pub trait Exporter {
    fn run(&mut self);
    fn get_options() -> HashMap<String, ExporterOption>;
}

pub struct ExporterOption {
    pub required: bool,
    pub takes_value: bool,
    pub default_value: String,
    pub possible_values: Vec<String>,
    pub short: String,
    pub long: String,
    pub value: String,
    pub help: String
}