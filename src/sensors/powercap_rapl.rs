use crate::sensors::units::Unit::MicroJoule;
use crate::sensors::utils::current_system_time_since_epoch;
use crate::sensors::{CPUSocket, Domain, Record, RecordReader, Sensor, Topology};
use procfs::{modules, KernelModule};
use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::{env, fs};

/// This is a Sensor type that relies on powercap and rapl linux modules
/// to collect energy consumption from CPU sockets and RAPL domains
pub struct PowercapRAPLSensor {
    base_path: String,
    buffer_per_socket_max_kbytes: u16,
    buffer_per_domain_max_kbytes: u16,
    virtual_machine: bool,
}

impl PowercapRAPLSensor {
    /// Instantiates and returns an instance of PowercapRAPLSensor.
    pub fn new(
        buffer_per_socket_max_kbytes: u16,
        buffer_per_domain_max_kbytes: u16,
        virtual_machine: bool,
    ) -> PowercapRAPLSensor {
        let mut powercap_path = String::from("/sys/class/powercap");
        if virtual_machine {
            powercap_path = String::from("/var/scaphandre");
            if let Ok(val) = env::var("SCAPHANDRE_POWERCAP_PATH") {
                powercap_path = val;
            }

            info!("Powercap_rapl path is: {}", powercap_path);
        }

        PowercapRAPLSensor {
            base_path: powercap_path,
            buffer_per_socket_max_kbytes,
            buffer_per_domain_max_kbytes,
            virtual_machine,
        }
    }

    /// Checks if intel_rapl modules are present and activated.
    pub fn check_module() -> Result<String, String> {
        let modules = modules().unwrap();
        let rapl_modules = modules
            .iter()
            .filter(|(_, v)| {
                v.name == "intel_rapl"
                    || v.name == "intel_rapl_msr"
                    || v.name == "intel_rapl_common"
            })
            .collect::<HashMap<&String, &KernelModule>>();

        if !rapl_modules.is_empty() {
            Ok(String::from(
                "intel_rapl or intel_rapl_msr+intel_rapl_common modules found.",
            ))
        } else {
            Err(String::from(
                "None of intel_rapl, intel_rapl_common or intel_rapl_msr kernel modules found.",
            ))
        }
    }
}

impl RecordReader for Topology {
    fn read_record(&self) -> Result<Record, Box<dyn Error>> {
        Ok(Record {
            timestamp: current_system_time_since_epoch(),
            value: String::from("5"),
            unit: MicroJoule,
        })
    }
}
impl RecordReader for CPUSocket {
    fn read_record(&self) -> Result<Record, Box<dyn Error>> {
        let source_file = self.sensor_data.get("source_file").unwrap();
        match fs::read_to_string(source_file) {
            Ok(result) => Ok(Record::new(
                current_system_time_since_epoch(),
                result,
                MicroJoule,
            )),
            Err(error) => Err(Box::new(error)),
        }
    }
}
impl RecordReader for Domain {
    fn read_record(&self) -> Result<Record, Box<dyn Error>> {
        let source_file = self.sensor_data.get("source_file").unwrap();
        match fs::read_to_string(source_file) {
            Ok(result) => Ok(Record {
                timestamp: current_system_time_since_epoch(),
                unit: MicroJoule,
                value: result,
            }),
            Err(error) => Err(Box::new(error)),
        }
    }
}

impl Sensor for PowercapRAPLSensor {
    /// Creates a Topology instance.
    fn generate_topology(&self) -> Result<Topology, Box<dyn Error>> {
        let modules_state = PowercapRAPLSensor::check_module();
        if modules_state.is_err() && !self.virtual_machine {
            warn!("Couldn't find intel_rapl modules.");
        }
        let mut topo = Topology::new();
        let re_socket = Regex::new(r"^.*/intel-rapl:\d+$").unwrap();
        let re_domain = Regex::new(r"^.*/intel-rapl:\d+:\d+$").unwrap();
        let mut re_domain_matched = false;
        for folder in fs::read_dir(&self.base_path).unwrap() {
            let folder_name = String::from(folder.unwrap().path().to_str().unwrap());
            // let's catch domain folders
            if re_domain.is_match(&folder_name) {
                re_domain_matched = true;
                // let's get the second number of the intel-rapl:X:X string
                let mut splitted = folder_name.split(':');
                let _ = splitted.next();
                let socket_id = String::from(splitted.next().unwrap()).parse().unwrap();
                let domain_id = String::from(splitted.next().unwrap()).parse().unwrap();
                let mut sensor_data_for_socket = HashMap::new();
                sensor_data_for_socket.insert(
                    String::from("source_file"),
                    format!("{}/intel-rapl:{}/energy_uj", self.base_path, socket_id),
                );
                topo.safe_add_socket(
                    socket_id,
                    vec![],
                    vec![],
                    format!("{}/intel-rapl:{}/energy_uj", self.base_path, socket_id),
                    self.buffer_per_socket_max_kbytes,
                    sensor_data_for_socket,
                );
                let mut sensor_data_for_domain = HashMap::new();
                sensor_data_for_domain.insert(
                    String::from("source_file"),
                    format!(
                        "{}/intel-rapl:{}:{}/energy_uj",
                        self.base_path, socket_id, domain_id
                    ),
                );
                if let Ok(domain_name) = &fs::read_to_string(format!("{folder_name}/name")) {
                    topo.safe_add_domain_to_socket(
                        socket_id,
                        domain_id,
                        domain_name.trim(),
                        &format!(
                            "{}/intel-rapl:{}:{}/energy_uj",
                            self.base_path, socket_id, domain_id
                        ),
                        self.buffer_per_domain_max_kbytes,
                        sensor_data_for_domain,
                    );
                }
            }
        }
        if !re_domain_matched {
            warn!("Couldn't find domain folders from powercap. Fallback on socket folders.");
            warn!("Scaphandre will not be able to provide per-domain data.");
            for folder in fs::read_dir(&self.base_path).unwrap() {
                let folder_name = String::from(folder.unwrap().path().to_str().unwrap());
                if re_socket.is_match(&folder_name) {
                    let mut splitted = folder_name.split(':');
                    let _ = splitted.next();
                    let socket_id = String::from(splitted.next().unwrap()).parse().unwrap();
                    let mut sensor_data_for_socket = HashMap::new();
                    sensor_data_for_socket.insert(
                        String::from("source_file"),
                        format!("{}/intel-rapl:{}/energy_uj", self.base_path, socket_id),
                    );
                    topo.safe_add_socket(
                        socket_id,
                        vec![],
                        vec![],
                        format!("{}/intel-rapl:{}/energy_uj", self.base_path, socket_id),
                        self.buffer_per_socket_max_kbytes,
                        sensor_data_for_socket,
                    )
                }
            }
        }
        topo.add_cpu_cores();
        Ok(topo)
    }

    /// Instanciates Topology object if not existing and returns it
    fn get_topology(&mut self) -> Box<Option<Topology>> {
        let topology = self.generate_topology().ok();
        if topology.is_none() {
            panic!("Couldn't generate the topology !");
        }
        Box::new(topology)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::type_name;

    fn type_of<T>(_: T) -> &'static str {
        type_name::<T>()
    }
    #[test]
    fn get_topology_returns_topology_type() {
        let mut sensor = PowercapRAPLSensor::new(1, 1, false);
        let topology = sensor.get_topology();
        assert_eq!(
            "alloc::boxed::Box<core::option::Option<scaphandre::sensors::Topology>>",
            type_of(topology)
        )
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
