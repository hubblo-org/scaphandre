//! Generic sensor and transmission agent for energy consumption related metrics.

use clap::{command, ArgAction, Parser, Subcommand};
use colored::Colorize;
use scaphandre::{exporters, sensors::Sensor};

#[cfg(target_os = "linux")]
use scaphandre::sensors::powercap_rapl;

#[cfg(target_os = "windows")]
use scaphandre::sensors::msr_rapl;

#[cfg(target_os = "windows")]
use windows_service::{
    service::ServiceControl,
    service::ServiceControlAccept,
    service::ServiceExitCode,
    service::ServiceState,
    service::ServiceStatus,
    service::ServiceType,
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};

#[cfg(target_os = "windows")]
define_windows_service!(ffi_service_main, my_service_main);

#[cfg(target_os = "windows")]
#[macro_use]
extern crate windows_service;

#[cfg(target_os = "windows")]
use std::time::Duration;

#[cfg(target_os = "windows")]
use std::ffi::OsString;

// the struct below defines the main Scaphandre command-line interface
/// Extensible metrology agent for electricity consumption related metrics.
#[derive(Parser)]
#[command(author, version)]
struct Cli {
    /// The exporter module to use to output the energy consumption metrics
    #[command(subcommand)]
    exporter: ExporterChoice,

    /// Increase the verbosity level
    #[arg(short, action = ArgAction::Count, default_value_t = 0)]
    verbose: u8,

    /// Don't print the header to the standard output
    #[arg(long, default_value_t = false)]
    no_header: bool,

    /// Tell Scaphandre that it's running in a virtual machine.
    /// You should have another instance of Scaphandre running on the hypervisor (see docs).
    #[arg(long, default_value_t = false)]
    vm: bool,

    /// The sensor module to use to gather the energy consumption metrics
    #[arg(short, long)]
    sensor: Option<String>,

    /// Maximum memory size allowed, in KiloBytes, for storing energy consumption of each **domain**.
    /// Only available for the RAPL sensor (on Linux).
    #[cfg(target_os = "linux")]
    #[arg(long, default_value_t = powercap_rapl::DEFAULT_BUFFER_PER_DOMAIN_MAX_KBYTES)]
    sensor_buffer_per_domain_max_kb: u16,

    /// Maximum memory size allowed, in KiloBytes, for storing energy consumption of each **socket**.
    /// Only available for the RAPL sensor (on Linux).
    #[cfg(target_os = "linux")]
    #[arg(long, default_value_t = powercap_rapl::DEFAULT_BUFFER_PER_SOCKET_MAX_KBYTES)]
    sensor_buffer_per_socket_max_kb: u16,
}

/// Defines the possible subcommands, one per exporter.
///
/// ### Description style
/// Per the clap documentation, the description of commands and arguments should be written in the style applied here,
/// *not* in the third-person. That is, use "Do xyz" instead of "Does xyz".
#[derive(Subcommand)]
enum ExporterChoice {
    /// Write the metrics to the terminal
    Stdout(exporters::stdout::ExporterArgs),

    /// Write the metrics in the JSON format to a file or to stdout
    #[cfg(feature = "json")]
    Json(exporters::json::ExporterArgs),

    /// Expose the metrics to a Prometheus HTTP endpoint
    #[cfg(feature = "prometheus")]
    Prometheus(exporters::prometheus::ExporterArgs),

    /// Watch all Qemu-KVM virtual machines running on the host and expose the metrics
    /// of each of them in a dedicated folder
    #[cfg(feature = "qemu")]
    Qemu,

    /// Expose the metrics to a Riemann server
    #[cfg(feature = "riemann")]
    Riemann(exporters::riemann::ExporterArgs),

    /// Expose the metrics to a Warp10 host, through HTTP
    #[cfg(feature = "warpten")]
    Warpten(exporters::warpten::ExporterArgs),

    /// Push metrics to Prometheus Push Gateway
    #[cfg(feature = "prometheuspush")]
    PrometheusPush(exporters::prometheuspush::ExporterArgs),
}

#[cfg(target_os = "windows")]
fn my_service_main(_arguments: Vec<OsString>) {
    use std::thread::JoinHandle;
    let graceful_period = 3;

    let start_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS, // Should match the one from system service registry
        current_state: ServiceState::Running,   // The new state
        controls_accepted: ServiceControlAccept::STOP, // Accept stop events when running
        exit_code: ServiceExitCode::Win32(0), // Used to report an error when starting or stopping only, otherwise must be zero
        checkpoint: 0, // Only used for pending states, otherwise must be zero
        wait_hint: Duration::default(), // Only used for pending states, otherwise must be zero
        process_id: None, // Unused for setting status
    };
    let stop_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    };
    let stoppending_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::StopPending,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::from_secs(graceful_period),
        process_id: None,
    };

    let thread_handle: Option<JoinHandle<()>>;
    let mut _stop = false;
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        println!("Got service control event: {:?}", control_event);
        match control_event {
            ServiceControl::Stop => {
                // Handle stop event and return control back to the system.
                _stop = true;
                ServiceControlHandlerResult::NoError
            }
            // All services must accept Interrogate even if it's a no-op.
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    if let Ok(system_handler) = service_control_handler::register("scaphandre", event_handler) {
        // Tell the system that the service is running now and run it
        match system_handler.set_service_status(start_status.clone()) {
            Ok(status_set) => {
                println!(
                    "Starting main thread, service status has been set: {:?}",
                    status_set
                );
                thread_handle = Some(std::thread::spawn(move || {
                    parse_cli_and_run_exporter();
                }));
            }
            Err(e) => {
                panic!("Couldn't set Windows service status. Error: {:?}", e);
            }
        }
        loop {
            if _stop {
                // Wait for the thread to finnish, then end the current function
                match system_handler.set_service_status(stoppending_status.clone()) {
                    Ok(status_set) => {
                        println!("Stop status has been set for service: {:?}", status_set);
                        if let Some(thr) = thread_handle {
                            if thr.join().is_ok() {
                                match system_handler.set_service_status(stop_status.clone()) {
                                    Ok(laststatus_set) => {
                                        println!(
                                            "Scaphandre gracefully stopped: {:?}",
                                            laststatus_set
                                        );
                                    }
                                    Err(e) => {
                                        panic!(
                                            "Could'nt set Stop status on scaphandre service: {:?}",
                                            e
                                        );
                                    }
                                }
                            } else {
                                panic!("Joining the thread failed.");
                            }
                            break;
                        } else {
                            panic!("Thread handle was not initialized.");
                        }
                    }
                    Err(e) => {
                        panic!("Couldn't set Windows service status. Error: {:?}", e);
                    }
                }
            }
        }
    } else {
        panic!("Failed getting system_handle.");
    }
}

fn main() {
    #[cfg(target_os = "windows")]
    match service_dispatcher::start("Scaphandre", ffi_service_main) {
        Ok(_) => {}
        Err(e) => {
            println!("Couldn't start Windows service dispatcher. Got : {}", e);
        }
    }

    parse_cli_and_run_exporter();
}

fn parse_cli_and_run_exporter() {
    let cli = Cli::parse();
    loggerv::init_with_verbosity(cli.verbose.into()).expect("unable to initialize the logger");

    let sensor = build_sensor(&cli);
    let mut exporter = build_exporter(cli.exporter, &sensor);
    if !cli.no_header {
        print_scaphandre_header(exporter.kind());
    }

    exporter.run();
}

fn build_exporter(choice: ExporterChoice, sensor: &dyn Sensor) -> Box<dyn exporters::Exporter> {
    match choice {
        ExporterChoice::Stdout(args) => {
            Box::new(exporters::stdout::StdoutExporter::new(sensor, args))
        }
        #[cfg(feature = "json")]
        ExporterChoice::Json(args) => {
            Box::new(exporters::json::JsonExporter::new(sensor, args)) // keep this in braces
        }
        #[cfg(feature = "prometheus")]
        ExporterChoice::Prometheus(args) => {
            Box::new(exporters::prometheus::PrometheusExporter::new(sensor, args))
        }
        #[cfg(feature = "qemu")]
        ExporterChoice::Qemu => {
            Box::new(exporters::qemu::QemuExporter::new(sensor)) // keep this in braces
        }
        #[cfg(feature = "riemann")]
        ExporterChoice::Riemann(args) => {
            Box::new(exporters::riemann::RiemannExporter::new(sensor, args))
        }
        #[cfg(feature = "warpten")]
        ExporterChoice::Warpten(args) => {
            Box::new(exporters::warpten::Warp10Exporter::new(sensor, args))
        }
        #[cfg(feature = "prometheuspush")]
        ExporterChoice::PrometheusPush(args) => Box::new(
            exporters::prometheuspush::PrometheusPushExporter::new(sensor, args),
        ),
    }
    // Note that invalid choices are automatically turned into errors by `parse()` before the Cli is populated,
    // that's why they don't appear in this function.
}

/// Returns the sensor to use, given the command-line arguments.
/// Unless sensor-specific options are provided, this should return
/// the same thing as [`scaphandre::get_default_sensor`].
fn build_sensor(cli: &Cli) -> impl Sensor {
    #[cfg(target_os = "linux")]
    let rapl_sensor = || {
        powercap_rapl::PowercapRAPLSensor::new(
            cli.sensor_buffer_per_socket_max_kb,
            cli.sensor_buffer_per_domain_max_kb,
            cli.vm,
        )
    };

    #[cfg(target_os = "windows")]
    let msr_sensor_win = msr_rapl::MsrRAPLSensor::new;

    match cli.sensor.as_deref() {
        Some("powercap_rapl") => {
            #[cfg(target_os = "linux")]
            {
                rapl_sensor()
            }
            #[cfg(not(target_os = "linux"))]
            panic!("Invalid sensor: Scaphandre's powercap_rapl only works on Linux")
        }
        Some("msr") => {
            #[cfg(target_os = "windows")]
            {
                msr_sensor_win()
            }
            #[cfg(not(target_os = "windows"))]
            panic!("Invalid sensor: Scaphandre's msr only works on Windows")
        }
        Some(s) => panic!("Unknown sensor type {}", s),
        None => {
            #[cfg(target_os = "linux")]
            return rapl_sensor();

            #[cfg(target_os = "windows")]
            return msr_sensor_win();

            #[cfg(not(any(target_os = "linux", target_os = "windows")))]
            compile_error!("Unsupported target OS")
        }
    }
}

fn print_scaphandre_header(exporter_name: &str) {
    let title = format!("Scaphandre {exporter_name} exporter");
    println!("{}", title.red().bold());
    println!("Sending ⚡ metrics");
}

#[cfg(test)]
mod test {
    use super::*;

    const SUBCOMMANDS: &[&str] = &[
        "stdout",
        #[cfg(feature = "prometheus")]
        "prometheus",
        #[cfg(feature = "riemann")]
        "riemann",
        #[cfg(feature = "json")]
        "json",
        #[cfg(feature = "warpten")]
        "warpten",
        #[cfg(feature = "qemu")]
        "qemu",
    ];

    /// Test that `--help` works for Scaphandre _and_ for each subcommand.
    /// This also ensures that all the subcommands are properly defined, as Clap will check some constraints
    /// when trying to parse a subcommand (for instance, it will check that no two short options have the same name).
    #[test]
    fn test_help() {
        fn assert_shows_help(args: &[&str]) {
            match Cli::try_parse_from(args) {
                Ok(_) => panic!(
                    "The CLI didn't generate a help message for {args:?}, are the inputs correct?"
                ),
                Err(e) => assert_eq!(
                    e.kind(),
                    clap::error::ErrorKind::DisplayHelp,
                    "The CLI emitted an error for {args:?}:\n{e}"
                ),
            };
        }
        assert_shows_help(&["scaphandre", "--help"]);
        for cmd in SUBCOMMANDS {
            assert_shows_help(&["scaphandre", cmd, "--help"]);
        }
    }
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
