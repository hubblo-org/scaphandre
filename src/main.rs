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
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity.")
        )
        .arg(
            Arg::with_name("sensor")
                .value_name("sensor")
                .help("Sensor module to apply on the host to get energy consumption metrics.")
                .required(false)
                .takes_value(true)
                .default_value("powercap_rapl")
                .possible_values(&sensors)
                .short("s")
                .long("sensor")
        ).arg(
            Arg::with_name("sensor-buffer-per-domain-max-kB")
                .value_name("sensor-buffer-per-domain-max-kB")
                .help("Maximum memory size allowed, in KiloBytes, for storing energy consumption of each domain.")
                .required(false)
                .takes_value(true)
                .default_value("1")
        ).arg(
            Arg::with_name("sensor-buffer-per-socket-max-kB")
                .value_name("sensor-buffer-per-socket-max-kB")
                .help("Maximum memory size allowed, in KiloBytes, for storing energy consumption of each socket.")
                .required(false)
                .takes_value(true)
                .default_value("2")
        ).arg(
            Arg::with_name("vm")
                .value_name("vm")
                .help("Tell scaphandre if he is running in a virtual machine.")
                .long("vm")
                .required(false)
                .takes_value(false)
        );

    for exp in res {
        let mut subcmd = SubCommand::with_name(exp).about(
            match exp {
                "stdout" => "stdout exporter allows you to output the power consumption data in the terminal, either in human readable format, JSON, or prometheus exporters like syntax.",
                "prometheus" => "prometheus exporter exposes power consumption metrics on an http endpoint (/metrics is default) in prometheus accepted format.",
                _ => "Unknown exporter.",
            }
        );
            
        for (key, opt) in exporters_options.get(exp).unwrap().iter() {
            let arg = Arg::with_name(key) 
             .required(opt.required)
             .takes_value(opt.takes_value)
             .default_value(&opt.default_value)
             .short(&opt.short)
             .long(&opt.long)
             .help(&opt.help);
            subcmd = subcmd.arg(arg);
        }
        matches = matches.subcommand(subcmd);
    }
    run(matches.get_matches());
}