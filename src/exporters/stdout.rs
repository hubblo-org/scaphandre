use std::time::{Instant, Duration};
use std::thread;
use std::collections::HashMap;
use crate::exporters::{Exporter, ExporterOption};
use crate::sensors::{Sensor, Record, Topology, RecordGenerator, energy_records_to_power_record};


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
        let mut records: Vec<Record> = vec![];
        let some_topology = *self.sensor.get_topology(); //Box<Option<&Topology>>
        let mut topology = some_topology.unwrap();
        if self.timeout.len() == 0 {
            self.iteration(topology, records);
        } else {
            let now = Instant::now();

            let timeout_secs: u64 = self.timeout.parse().unwrap();
            let step = 2;

            while now.elapsed().as_secs() <= timeout_secs {
                println!("Step: {}s", step);
                let result = self.iteration(topology, records);
                topology = result.0;
                records = result.1;
                thread::sleep(Duration::new(step, 0)); 
            }
        }
    }
    fn iteration(&mut self, mut topology: Topology, mut records: Vec<Record>) -> (Topology, Vec<Record>){
        //topology = Option<&Topology>
        for socket in topology.get_sockets() {
            let socket_id = socket.id;
            records.push(socket.get_record());
            let mut power = String::from("unknown");
            let mut unit = String::from("W");
            let nb_records = records.len();
            if nb_records > 1 {
                let power_record = &energy_records_to_power_record(
                   (
                       records.get(nb_records - 1).unwrap(),
                       records.get(nb_records - 2).unwrap()
                   )
                ).unwrap();
                power = power_record.value.clone();
                unit = power_record.unit.to_string();
            }
            let mut rec_j_1 = String::from("unknown");
            let mut rec_j_2 = String::from("unknown");
            let mut rec_j_3 = String::from("unknown");
            if records.len() > 2 { rec_j_1 = records.get(nb_records - 3).unwrap().value.to_string(); }
            if records.len() > 1 { rec_j_2 = records.get(nb_records - 2).unwrap().value.to_string(); }
            if records.len() > 0 { rec_j_3 = records.get(nb_records - 1).unwrap().value.to_string(); }
            println!(
                "socket:{} {} {} last3(uJ): {} {} {}",
                socket_id, power, unit, rec_j_1, rec_j_2, rec_j_3
            );

            for domain in socket.get_domains() {
            //    println!(
            //        "socket {} | domain {} {} | counter (uJ) {}",
            //        socket_id, domain.id, domain.name, domain.read_counter_uj().unwrap()
            //    );
            //    println!(
            //        "{}", domain.get_record()
            //    );
            }
        }
        (topology, records)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn get_cons_socket0(){

    }
}