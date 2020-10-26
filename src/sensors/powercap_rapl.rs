use std::error::Error;
use std::fs;
use regex::Regex;
use crate::sensors::Sensor as Sensor;
use crate::sensors::Topology as Topology;
use crate::sensors::Record as Record;

pub struct PowercapRAPLSensor {
    base_path: String,
    topology: Option<Topology>,
}

impl PowercapRAPLSensor {
    pub fn new<'a>() -> PowercapRAPLSensor {
        PowercapRAPLSensor{
            base_path: String::from("/sys/class/powercap"),
            topology: None
        }
    }
}

impl Sensor for PowercapRAPLSensor {
    fn generate_topology(&self) -> Result<Topology, Box<dyn Error>> {
        let mut topo = Topology::new();
        let re_domain = Regex::new(r"^.*/intel-rapl:\d+:\d+$").unwrap();
        for folder in fs::read_dir(&self.base_path).unwrap(){
            let folder_name =  String::from(folder.unwrap().path().to_str().unwrap());
            // let's catch domain folders
            if re_domain.is_match(&folder_name) {                    
                // let's get the second number of the intel-rapl:X:X string
                let mut splitted = folder_name.split(":");
                let _ = splitted.next();
                let socket_id = String::from(splitted.next().unwrap()).parse().unwrap();
                let domain_id = String::from(splitted.next().unwrap()).parse().unwrap();
                topo.safe_add_socket(
                    socket_id, vec![], vec![],
                    format!("{}/intel-rapl:{}/energy_uj", self.base_path, socket_id)
                );
                topo.safe_add_domain_to_socket(
                    socket_id, domain_id,
                    &fs::read_to_string(format!("{}/name", folder_name)).unwrap(),
                    &format!("{}/intel-rapl:{}:{}/energy_uj", self.base_path, socket_id, domain_id)
                );
            }
        }
        Ok(topo)
    }

    fn get_topology(&mut self) -> Box<Option<&Topology>> {
        if self.topology.is_none() {
            println!("\n\nGenerating topology \n\n");
            self.topology = self.generate_topology().ok();                
            if self.topology.is_none() {
                eprintln!("Couldn't generate the topology !");
            }
        }
        Box::new(self.topology.as_ref())
    }

    //fn get_record(&self) -> Record {
    //    Record::new(0, 0, )
    //}
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
        let mut sensor = PowercapRAPLSensor::new();
        let topology = sensor.get_topology();
        assert_eq!("alloc::boxed::Box<core::option::Option<&scaphandre::sensors::Topology>>", type_of(topology))
    }

}