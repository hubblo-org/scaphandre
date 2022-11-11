//! # ElasticExporter
//!
//! `ElasticExporter` implementation, exposes metrics to
//! an [ElasticSearch](https://www.elastic.co/fr/elasticsearch/) server.

use crate::exporters::Exporter;
use crate::sensors::Sensor;
use clap::{Arg, ArgMatches};
use elasticsearch::{
    auth::Credentials,
    http::transport::{SingleNodeConnectionPool, Transport, TransportBuilder},
    CreateParts, Elasticsearch, Error,
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use url::Url;

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
    fn run(&mut self, parameters: ArgMatches) {
        let client = match new_client(
            parameters.value_of("scheme").unwrap(),
            parameters.value_of("host").unwrap(),
            parameters.value_of("port").unwrap(),
            parameters.value_of("cloud_id"),
            parameters.value_of("username"),
            parameters.value_of("password"),
        ) {
            Ok(client) => client,
            Err(e) => panic!("{}", e),
        };

        if let Err(e) = self.runner(client) {
            error!("{}", e)
        }
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

const ES_INDEX_NAME: &str = "scaphandre";

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ScaphandreData {
    pub wip: i32,
}

impl ElasticExporter {
    /// Instantiates and returns a new ElasticExporter
    // TODO: make sensor mutable
    pub fn new(sensor: Box<dyn Sensor>) -> ElasticExporter {
        ElasticExporter { _sensor: sensor }
    }

    #[tokio::main]
    pub async fn runner(&self, client: Elasticsearch) -> Result<(), Error> {
        self.ensure_index(&client).await?;

        // WIP
        let create_test_resp = client
            .create(CreateParts::IndexId(ES_INDEX_NAME, "42"))
            .body(ScaphandreData { wip: 42 })
            .send()
            .await?;

        println!("create test resp {}", create_test_resp.status_code());

        Ok(())
    }

    async fn ensure_index(&self, client: &Elasticsearch) -> Result<(), Error> {
        let index_exist_resp = client
            .indices()
            .exists(elasticsearch::indices::IndicesExistsParts::Index(&[
                ES_INDEX_NAME,
            ]))
            .send()
            .await?;

        if index_exist_resp.status_code() == StatusCode::OK {
            return Ok(());
        }

        let index_create_resp = client
            .indices()
            .create(elasticsearch::indices::IndicesCreateParts::Index(
                ES_INDEX_NAME,
            ))
            .send()
            .await?;

        // WIP
        if !index_create_resp.status_code().is_success() {
            println!(
                "Error while creating index: status_code {}",
                index_create_resp.status_code()
            )
        }

        Ok(())
    }
}

/// Inits a new elastic client
fn new_client(
    scheme: &str,
    host: &str,
    port: &str,
    cloud_id: Option<&str>,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<Elasticsearch, Error> {
    let credentials = match (username, password) {
        (Some(u), Some(p)) => Some(Credentials::Basic(u.to_string(), p.to_string())),
        _ => None,
    };

    let transport = match (credentials, cloud_id) {
        (Some(credentials), Some(cloud_id)) => Transport::cloud(cloud_id, credentials)?,
        (Some(credentials), None) => {
            let url = Url::parse(&format_url(scheme, host, port))?;
            let conn = SingleNodeConnectionPool::new(url);
            TransportBuilder::new(conn).auth(credentials).build()?
        }
        (None, None) => Transport::single_node(&format_url(scheme, host, port))?,
        _ => unreachable!(),
    };

    Ok(Elasticsearch::new(transport))
}

/// Format an url to an elastic endpoint
fn format_url<'a>(scheme: &'a str, host: &'a str, port: &'a str) -> String {
    format!("{scheme}://{host}:{port}")
}
