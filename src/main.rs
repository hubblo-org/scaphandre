//! Generic sensor and transmission agent for energy consumption related metrics. 
//! 
use clap::{Arg, App, SubCommand, AppSettings};
use scaphandre::{run, get_exporters_options};

fn main() {
    let sensors = ["powercap_rapl"];
    let exporters_options = get_exporters_options();
    let exporters = exporters_options.keys();
    let mut res = vec![];
    for i in exporters {
        res.push(i.as_str());
    }
    let mut matches = App::new("scaphandre")
        .author("Benoit Petit <bpetit@hubblo.org>")
        .version("0.1.0")
        .about("Agnostic software sensor and data collection agent for energy/electricity consumption related metrics")
        .setting(AppSettings::SubcommandRequiredElseHelp)
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
        );
    for exp in res {
        let mut subcmd = SubCommand::with_name(exp);
        for (key, opt) in exporters_options.get(exp).unwrap().iter() {
            subcmd = subcmd.arg(                
               Arg::with_name(key) 
                .required(opt.required)
                .takes_value(opt.takes_value)
                .default_value(&opt.default_value)
                .short(&opt.short)
                .long(&opt.long)
            );
        }
        matches = matches.subcommand(subcmd);
    }
    run(matches.get_matches());
}