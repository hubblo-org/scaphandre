//! Generic sensor and transmission agent for energy consumption related metrics. 
//! 
use clap::{Arg, App};
use scaphandre::run;
fn main() {
    let sensors = ["powercap_rapl"];
    let exporters = ["stdout"];
    let matches = App::new("gitmeup")
        .author("Benoit Petit <bpetit@hubblo.org>")
        .version("0.1.0")
        .about("Agnostic software sensor and data collection agent for energy/electricity consumption related metrics")
        .arg(
            Arg::with_name("sensor")
                .value_name("sensor")
                .help("The sensor module to apply on the host to get energy consumption metrics.")
                .required(true)
                .takes_value(true)
                .default_value("powercap_rapl")
                .possible_values(&sensors)
                .short("s")
                .long("sensor")
        ).arg(
            Arg::with_name("exporter")
                .value_name("exporter")
                .help("The exporter module to apply on the host to get energy consumption metrics.")
                .required(true)
                .takes_value(true)
                .possible_values(&exporters)
                .default_value("stdout")
                .short("e")
                .long("exporter")
        )
        .get_matches();
    run(matches);
}