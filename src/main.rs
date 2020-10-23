//! Generic sensor and transmission agent for energy consumption related metrics. 
//! 
use clap::{Arg, App};
use scaphandre::run;
fn main() {
    let sensors = ["powercap_rapl"];
    let matches = App::new("gitmeup")
        .author("Benoit Petit <bpetit@hubblo.org>")
        .version("0.1.0")
        .about("Agnostic software sensor and data collection agent for energy/electricity consumption related metrics")
        .arg(
            Arg::with_name("sensor")
                .value_name("sensor")
                .help("The sensor strategy to apply on the host to get energy consumption metrics.")
                .required(true)
                .takes_value(true)
                .default_value("powercap_rapl")
                .possible_values(&sensors)
                .short("s")
                .long("sensor")
        ).get_matches();
    if matches.value_of("sensor").unwrap() == "powercap_rapl" {
        run();
    }
}