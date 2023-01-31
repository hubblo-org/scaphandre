use crate::exporters::Exporter;
use crate::sensors::{utils::ProcessRecord, Sensor, Topology};
use std::{fs, io, thread, time};

/// An Exporter that extracts power consumption data of running
/// Qemu/KVM virtual machines on the host and store those data
/// as folders and files that are supposed to be mounted on the
/// guest/virtual machines. This allow users of the virtual machines
/// to collect and deal with their power consumption metrics, the same way
/// they would do it if they managed bare metal machines.
pub struct QemuExporter {
    topology: Topology,
}

impl Exporter for QemuExporter {
    /// Runs iteration() in a loop.
    fn run(&mut self, _parameters: clap::ArgMatches) {
        info!("Starting qemu exporter");
        let path = "/var/lib/libvirt/scaphandre";
        let cleaner_step = 120;
        let mut timer = time::Duration::from_secs(cleaner_step);
        loop {
            self.iteration(String::from(path));
            let step = time::Duration::from_secs(5);
            thread::sleep(step);
            if timer - step > time::Duration::from_millis(0) {
                timer -= step;
            } else {
                self.topology
                    .proc_tracker
                    .clean_terminated_process_records_vectors();
                timer = time::Duration::from_secs(cleaner_step);
            }
        }
    }

    fn get_options() -> Vec<clap::Arg<'static, 'static>> {
        Vec::new()
    }
}

impl QemuExporter {
    /// Instantiates and returns a new QemuExporter
    pub fn new(mut sensor: Box<dyn Sensor>) -> QemuExporter {
        let some_topology = *sensor.get_topology();
        QemuExporter {
            topology: some_topology.unwrap(),
        }
    }

    /// Performs processing of metrics, using self.topology
    pub fn iteration(&mut self, path: String) {
        trace!("path: {}", path);
        self.topology.refresh();
        let topo_uj_diff = self.topology.get_records_diff();
        let topo_stat_diff = self.topology.get_stats_diff();
        if let Some(topo_rec_uj) = topo_uj_diff {
            debug!("Got topo uj diff: {:?}", topo_rec_uj);
            let proc_tracker = self.topology.get_proc_tracker();
            let processes = proc_tracker.get_alive_processes();
            let qemu_processes = QemuExporter::filter_qemu_vm_processes(&processes);
            debug!(
                "Number of filtered qemu processes: {}",
                qemu_processes.len()
            );
            for qp in qemu_processes {
                info!("Working on {:?}", qp);
                if qp.len() > 2 {
                    let last = qp.first().unwrap();
                    let previous = qp.get(1).unwrap();
                    let vm_name = QemuExporter::get_vm_name_from_cmdline(
                        &last.process.original.cmdline().unwrap(),
                    );
                    let time_pdiff = last.total_time_jiffies() - previous.total_time_jiffies();
                    if let Some(time_tdiff) = &topo_stat_diff {
                        let first_domain_path = format!("{path}/{vm_name}/intel-rapl:0:0");
                        if fs::read_dir(&first_domain_path).is_err() {
                            match fs::create_dir_all(&first_domain_path) {
                                Ok(_) => info!("Created {} folder.", &path),
                                Err(error) => panic!("Couldn't create {}. Got: {}", &path, error),
                            }
                        }
                        let tdiff = time_tdiff.total_time_jiffies();
                        trace!("Time_pdiff={} time_tdiff={}", time_pdiff.to_string(), tdiff);
                        let ratio = time_pdiff / tdiff;
                        trace!("Ratio is {}", ratio.to_string());
                        let uj_to_add = ratio * topo_rec_uj.value.parse::<u64>().unwrap();
                        trace!("Adding {} uJ", uj_to_add);
                        let complete_path = format!("{path}/{vm_name}/intel-rapl:0");
                        if let Ok(result) = QemuExporter::add_or_create(&complete_path, uj_to_add) {
                            trace!("{:?}", result);
                            debug!("Updated {}", complete_path);
                        }
                    }
                }
            }
        }
    }

    /// Parses a cmdline String (as contained in procs::Process instances) and returns
    /// the name of the qemu virtual machine if this process is a qemu/kvm guest process
    fn get_vm_name_from_cmdline(cmdline: &[String]) -> String {
        for elmt in cmdline {
            if elmt.starts_with("guest=") {
                let mut splitted = elmt.split('=');
                splitted.next();
                return String::from(splitted.next().unwrap().split(',').next().unwrap());
            }
        }
        String::from("")
    }

    /// Either creates an energy_uj file (as the ones managed by powercap kernel module)
    /// in 'path' and adds 'uj_value' to its numerical content, or simply performs the
    /// addition if the file exists.
    fn add_or_create(path: &str, uj_value: u64) -> io::Result<()> {
        let mut content = 0;
        if fs::read_dir(path).is_err() {
            match fs::create_dir_all(path) {
                Ok(_) => info!("Created {} folder.", path),
                Err(error) => panic!("Couldn't create {}. Got: {}", path, error),
            }
        }
        let file_path = format!("{}/{}", path, "energy_uj");
        if let Ok(file) = fs::read_to_string(&file_path) {
            content = file.parse::<u64>().unwrap();
            content += uj_value;
        }
        fs::write(file_path, content.to_string())
    }

    /// Filters 'processes' to match processes that look like qemu/kvm guest processes.
    /// Returns what was found.
    fn filter_qemu_vm_processes(processes: &[&Vec<ProcessRecord>]) -> Vec<Vec<ProcessRecord>> {
        let mut qemu_processes: Vec<Vec<ProcessRecord>> = vec![];
        trace!("Got {} processes to filter.", processes.len());
        for vecp in processes.iter() {
            if !vecp.is_empty() {
                if let Some(pr) = vecp.get(0) {
                    if let Ok(cmdline) = pr.process.original.cmdline() {
                        if let Some(res) = cmdline.iter().find(|x| x.contains("qemu-system")) {
                            debug!("Found a process with {}", res);
                            let mut tmp: Vec<ProcessRecord> = vec![];
                            for p in vecp.iter() {
                                tmp.push(p.clone());
                            }
                            qemu_processes.push(tmp);
                        }
                    }
                }
            }
        }
        qemu_processes
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
