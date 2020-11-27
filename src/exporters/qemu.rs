use crate::exporters::{Exporter};
use crate::sensors::{Sensor, Topology, utils::ProcessRecord};
use std::collections::HashMap;
use std::{fs, io, thread, time};
use regex::Regex;

/// An Exporter that extracts power consumption data of running
/// Qemu/KVM virtual machines on the host and store those data
/// as folders and files that are supposed to be mounted on the
/// guest/virtual machines. This allow users of the virtual machines
/// to collect and deal with their power consumption metrics, the same way
/// they would do it if they managed bare metal machines.
pub struct QemuExporter {
    topology: Topology
}

impl Exporter for QemuExporter {
    fn run(&mut self, parameters: clap::ArgMatches) {
        info!("Starting qemu exporter");    
        let stop = false;
        let path = "/var/lib/libvirt/scaphandre";
        while !stop {
            self.iteration(String::from(path));
            thread::sleep(time::Duration::from_secs(5));
        }
    }

    fn get_options() -> HashMap<String, super::ExporterOption> {
       HashMap::new() 
    }
}

impl QemuExporter {
    /// Instantiates and returns a new QemuExporter
    pub fn new(mut sensor: Box<dyn Sensor>) -> QemuExporter {
        let some_topology = *sensor.get_topology();
        QemuExporter { topology: some_topology.unwrap() }
    }

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
            info!("Number of filtered qemu processes: {}", qemu_processes.len());
            for qp in qemu_processes {
                info!("Working on {:?}", qp);
                if qp.len() > 2 {
                    let last = qp.first().unwrap();
                    let previous = qp.get(1).unwrap();
                    let vm_name = QemuExporter::get_vm_name_from_cmdline(&last.process.cmdline().unwrap());
                    let time_pdiff = last.total_time_jiffies() - previous.total_time_jiffies();
                    if let Some(time_tdiff) = &topo_stat_diff {
                        let first_domain_path = format!("{}/{}/intel-rapl:0:0", path, vm_name);
                        if fs::read_dir(&first_domain_path).is_err() {
                            match fs::create_dir_all(&first_domain_path){
                                Ok(res) => info!("Created {} folder.", &path),
                                Err(error) => panic!("Couldn't create {}. Got: {}", &path, error)
                            }
                        }
                        let ratio = time_pdiff as f32 / &time_tdiff.total_time_jiffies();
                        let uj_to_add = ratio * topo_rec_uj.value.parse::<f32>().unwrap();
                        let complete_path = format!("{}/{}/intel-rapl:0", path, vm_name);
                        if let Ok(result) = QemuExporter::add_or_create(
                            &complete_path, uj_to_add as u64
                        ) {
                            trace!("{:?}", result);
                            debug!("Updated {}", complete_path);
                        }
                    }
                }
            }
        }
    }

    fn get_vm_name_from_cmdline(cmdline: &Vec<String>) -> String{
        for elmt in cmdline {
            if elmt.starts_with("guest=") {                    
                let mut splitted = elmt.split('=');
                splitted.next();
                return String::from(splitted.next().unwrap().split(',').next().unwrap());
            }
        }
        String::from("")
    }

    fn add_or_create(path: &str, uj_value: u64) -> io::Result<()> {
        let mut content = 0;
        if fs::read_dir(path).is_err() {
            match fs::create_dir_all(path){
                Ok(res) => info!("Created {} folder.", path),
                Err(error) => panic!("Couldn't create {}. Got: {}", path, error)
            }
        }
        let file_path = format!("{}/{}", path, "energy_uj");
        if let Ok(file) = fs::read_to_string(&file_path) {                
            content = file.parse::<u64>().unwrap();
            content += uj_value;
        }
        fs::write(file_path, content.to_string())
    }

    fn filter_qemu_vm_processes(processes: &Vec<&Vec<ProcessRecord>>) -> Vec<Vec<ProcessRecord>>{
        let mut qemu_processes: Vec<Vec<ProcessRecord>> = vec![];
        trace!("Got {} processes to filter.", processes.len());
        for vecp in processes.iter() {
            if !vecp.is_empty() {
                if let Some(pr) = vecp.get(0) {
                    if let Ok(cmdline) = pr.process.cmdline() {
                        if let Some(res) = cmdline.iter().filter(|x| x.contains("qemu-system")).next() {
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
