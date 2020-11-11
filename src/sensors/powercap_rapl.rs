use std::error::Error;
use std::fs;
use std::collections::HashMap;
use regex::Regex;
use crate::sensors::Sensor as Sensor;
use crate::sensors::Topology as Topology;
use procfs::{modules, KernelModule};

pub struct PowercapRAPLSensor {
    base_path: String,
    buffer_per_socket_max_kbytes: u16,
    buffer_per_domain_max_kbytes: u16,
}

impl PowercapRAPLSensor {
    pub fn new(
        buffer_per_socket_max_kbytes: u16, buffer_per_domain_max_kbytes: u16
    ) -> PowercapRAPLSensor {
        PowercapRAPLSensor{
            base_path: String::from("/sys/class/powercap"),
            buffer_per_socket_max_kbytes,
            buffer_per_domain_max_kbytes
        }
    }

    /// Checks if intel_rapl modules are present and activated.
    pub fn check_module() -> Result<String, String> {
        let modules = modules().unwrap();
        let rapl_modules = modules.iter().filter(
            |(_, v)| v.name == "intel_rapl_msr" || v.name == "intel_rapl_common"
        ).collect::<HashMap<&String, &KernelModule>>();

        if rapl_modules.len() > 1 {
            Ok(String::from("intel_rapl_msr and intel_rapl_common modules found."))
        } else {
            Err(String::from("intel_rapl_common and intel_rapl_msr kernel modules not found."))
        }
    }
}

impl Sensor for PowercapRAPLSensor {
    /// Creates a Topology instance.
    fn generate_topology(&self) -> Result<Topology, Box<dyn Error>> {
        let modules_state = PowercapRAPLSensor::check_module();
        if modules_state.is_err() {
            panic!("Couldn't find intel_rapl modules.");
        }
        let mut topo = Topology::new();
        let re_domain = Regex::new(r"^.*/intel-rapl:\d+:\d+$").unwrap();
        for folder in fs::read_dir(&self.base_path).unwrap(){
            let folder_name =  String::from(folder.unwrap().path().to_str().unwrap());
            // let's catch domain folders
            if re_domain.is_match(&folder_name) {                    
                // let's get the second number of the intel-rapl:X:X string
                let mut splitted = folder_name.split(':');
                let _ = splitted.next();
                let socket_id = String::from(splitted.next().unwrap()).parse().unwrap();
                let domain_id = String::from(splitted.next().unwrap()).parse().unwrap();
                topo.safe_add_socket(
                    socket_id, vec![], vec![],
                    format!("{}/intel-rapl:{}/energy_uj", self.base_path, socket_id),
                    self.buffer_per_socket_max_kbytes
                );
                topo.safe_add_domain_to_socket(
                    socket_id, domain_id,
                    &fs::read_to_string(format!("{}/name", folder_name)).unwrap(),
                    &format!("{}/intel-rapl:{}:{}/energy_uj", self.base_path, socket_id, domain_id),
                    self.buffer_per_domain_max_kbytes
                );
            }
        }
        topo.add_cpu_cores();
        Ok(topo)
    }

    fn get_topology(&mut self) -> Box<Option<Topology>> {
        let topology = self.generate_topology().ok();                
        if topology.is_none() {
            eprintln!("Couldn't generate the topology !");
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
        let mut sensor = PowercapRAPLSensor::new(1, 1);
        let topology = sensor.get_topology();
        assert_eq!("alloc::boxed::Box<core::option::Option<&scaphandre::sensors::Topology>>", type_of(topology))
    }

}