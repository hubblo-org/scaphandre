use std::time::{Instant, Duration};
use std::thread;
use std::collections::HashMap;
use crate::exporters::{Exporter, ExporterOption};
use crate::sensors::{Sensor, RecordGenerator};


pub struct StdoutExporter {
    sensor: Box<dyn Sensor>,
    timeout: String
}

impl Exporter for StdoutExporter {
    fn run(&mut self) {
        self.runner();
    }

    fn get_options() -> HashMap<String, ExporterOption> {
        let mut options = HashMap::new();
        options.insert(
            String::from("timeout"),
            ExporterOption{
                default_value: String::from(""),
                long: String::from("timeout"),
                short: String::from("t"),
                required: false,
                takes_value: true,
                possible_values: vec![],
                value: String::from("")
            }
        );
        options
    }
}

impl StdoutExporter {
    pub fn new(sensor: Box<dyn Sensor>, timeout: String) -> StdoutExporter {
        StdoutExporter { sensor, timeout }    
    }

    pub fn runner (&mut self) {
        if self.timeout.len() == 0 {
            self.iteration();
        } else {
            let now = Instant::now();

            let timeout_secs: u64 = self.timeout.parse().unwrap();
            while now.elapsed().as_secs() <= timeout_secs {
                self.iteration();
                thread::sleep(Duration::new(1, 0)); 
            }
        }
    }
    fn iteration(&mut self) {
        let topology = *self.sensor.get_topology(); //Box<Option<&Topology>>
        //topology = Option<&Topology>
        let topol = match topology {
            Some(topo) => topo, // topol = &Topology
            None => panic!("Topology has not been generated.")
        };
        for mut socket in topol.sockets {
            let socket_id = socket.id;
            println!(
                "socket {} | counter (uJ) {}",
                socket_id, socket.read_counter_uj().unwrap()
            );
            println!("{}", socket.get_record());
            for domain in socket.get_domains() {
                println!(
                    "socket {} | domain {} {} | counter (uJ) {}",
                    socket_id, domain.id, domain.name, domain.read_counter_uj().unwrap()
                );
                println!(
                    "{}", domain.get_record()
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