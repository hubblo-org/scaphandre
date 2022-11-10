//! # ElasticExporter
//!
//! `ElasticExporter` implementation, exposes metrics to
//! an [ElasticSearch](https://www.elastic.co/fr/elasticsearch/) server.

use crate::exporters::Exporter;
use crate::sensors::Sensor;
use clap::{Arg, ArgMatches};

/// Default url for Elastic endpoint
const DEFAULT_HOST: &str = "localhost";
/// Default port for Elastic endpoint
const DEFAULT_PORT: &str = "9200";
/// Default scheme for Elastic endpoint
const DEFAULT_SCHEME: &str = "http";

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
        let host = Arg::with_name("host")
            .default_value(DEFAULT_HOST)
            .help("FDQN used to join Elastic host")
            .long("host")
            .short("h")
            .required(false)
            .takes_value(true);

        let port = Arg::with_name("port")
            .default_value(DEFAULT_PORT)
            .help("TCP port used to join Elastic host")
            .long("port")
            .short("p")
            .required(false)
            .takes_value(true);

        let scheme = Arg::with_name("scheme")
            .default_value(DEFAULT_SCHEME)
            .help("URL scheme used to join Elastic host")
            .long("scheme")
            .short("s")
            .required(false)
            .takes_value(true);

        let cloud_id = Arg::with_name("cloud_id")
            .help("Cloud id for Elasticsearch deployment in Elastic Cloud")
            .long("cloudid")
            .short("c")
            .required(false)
            .takes_value(true);

        let username = Arg::with_name("username")
            .help("Basic auth username")
            .long("username")
            .short("U")
            .required(false)
            .takes_value(true);

        let password = Arg::with_name("password")
            .help("Basic auth password")
            .long("password")
            .short("P")
            .required(false)
            .takes_value(true);

        let qemu = Arg::with_name("qemu")
            .help("Tells scaphandre it is running on a Qemu hypervisor.")
            .long("qemu")
            .short("q")
            .required(false)
            .takes_value(false);

        let containers = Arg::with_name("containers")
            .help("Monitor and apply labels for processes running as containers")
            .long("containers")
            .short("C")
            .required(false)
            .takes_value(false);

        vec![
            host, port, scheme, cloud_id, username, password, qemu, containers,
        ]
    }
}
