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
            println!(
                "socket {} | counter {} µJ",
                socket.id,
                socket.read_counter_uj().unwrap()
            );
            for domain in &socket.domains {
                println!(
                    "socket {} | domain {} {} | counter {} µJ",
                    socket.id, domain.id, domain.name, domain.read_counter_uj().unwrap()
                );
            }
        }
    }


}

#[cfg(test)]
mod tests {
    #[test]
    fn get_cons_socket0(){

    }
}