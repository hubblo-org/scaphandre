use crate::sensors::Sensor;
use crate::sensors::Topology;
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

            warn!("Powercap_rapl path is: {}", powercap_path);
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

impl Sensor for PowercapRAPLSensor {
    /// Creates a Topology instance.
    fn generate_topology(&self) -> Result<Topology, Box<dyn Error>> {
        let modules_state = PowercapRAPLSensor::check_module();
        if modules_state.is_err() && !self.virtual_machine {
            warn!("Couldn't find intel_rapl modules.");
        }
        let mut topo = Topology::new();
        let re_domain = Regex::new(r"^.*/intel-rapl:\d+:\d+$").unwrap();
        for folder in fs::read_dir(&self.base_path).unwrap() {
            let folder_name = String::from(folder.unwrap().path().to_str().unwrap());
            // let's catch domain folders
            if re_domain.is_match(&folder_name) {
                // let's get the second number of the intel-rapl:X:X string
                let mut splitted = folder_name.split(':');
                let _ = splitted.next();
                let socket_id = String::from(splitted.next().unwrap()).parse().unwrap();
                let domain_id = String::from(splitted.next().unwrap()).parse().unwrap();
                topo.safe_add_socket(
                    socket_id,
                    vec![],
                    vec![],
                    format!("{}/intel-rapl:{}/energy_uj", self.base_path, socket_id),
                    self.buffer_per_socket_max_kbytes,
                );
                if let Ok(domain_name) = &fs::read_to_string(format!("{}/name", folder_name)) {
                    topo.safe_add_domain_to_socket(
                        socket_id,
                        domain_id,
                        domain_name.trim(),
                        &format!(
                            "{}/intel-rapl:{}:{}/energy_uj",
                            self.base_path, socket_id, domain_id
                        ),
                        self.buffer_per_domain_max_kbytes,
                    );
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
