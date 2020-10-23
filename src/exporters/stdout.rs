use crate::exporters::Exporter;
use crate::sensors::Sensor;

pub struct StdoutExporter {
    sensor: Box<dyn Sensor>,
}

impl StdoutExporter {
    pub fn new(sensor: Box<dyn Sensor>) -> StdoutExporter {
        StdoutExporter {
            sensor
        }    
    }
}

impl Exporter for StdoutExporter {
    fn run (&mut self) {
        let topology = *self.sensor.get_topology();
        let topology = match topology {
            Some(topo) => topo,
            None => panic!("Topology has not been generated.")
        };
        for socket in &topology.sockets {
            println!("Browsing socket {}", socket.id);
            println!(
                "Overall socket energy: {} µJ",
                socket.read_counter_uj().unwrap()
            );
            for domain in &socket.domains {
                println!("Browsing domain {} : {}", domain.id, domain.name);
                println!(
                    "Current energy counter value: {} µJ",
                    domain.read_counter_uj().unwrap()
                );
            }
        }
    }
}