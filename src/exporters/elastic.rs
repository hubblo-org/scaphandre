//! # ElasticExporter
//!
//! `ElasticExporter` implementation, exposes metrics to
//! an [ElasticSearch](https://www.elastic.co/fr/elasticsearch/) server.

use crate::exporters::Exporter;
use crate::sensors::Sensor;
use clap::ArgMatches;

/// Exporter that pushes metrics to an ElasticSearch endpoint
pub struct ElasticExporter {
    /// Sensor instance that is used to generate the Topology and
    /// thus get power consumption metrics.
    _sensor: Box<dyn Sensor>,
}

impl Exporter for ElasticExporter {
    fn run(&mut self, _parameters: ArgMatches) {
        // TODO
    }

    fn get_options() -> Vec<clap::Arg<'static, 'static>> {
        Vec::new()
    }
}
