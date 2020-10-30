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
                default_value: String::from("5"),
                long: String::from("timeout"),
                short: String::from("t"),
                required: false,
                takes_value: true,
                possible_values: vec![],
                value: String::from(""),
                help: String::from("Maximum time spent measuring, in seconds.")
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
            let step = 1;

            println!("Measurement step is: {}s", step);

            while now.elapsed().as_secs() <= timeout_secs {
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
            records.push(socket.refresh_record());
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
            if records.len() > 2 { rec_j_1 = records.get(nb_records - 3).unwrap().value.to_string().trim().to_string(); }
            if records.len() > 1 { rec_j_2 = records.get(nb_records - 2).unwrap().value.to_string().trim().to_string(); }
            if records.len() > 0 { rec_j_3 = records.get(nb_records - 1).unwrap().value.to_string().trim().to_string(); }
            println!(
                "socket:{} {} {} last3(uJ): {} {} {}",
                socket_id, power, unit, rec_j_1, rec_j_2, rec_j_3
            );

            for domain in socket.get_domains() {
                domain.refresh_record();
                let domain_records = domain.get_records_passive();
                let mut power = String::from("unknown");
                let mut unit = String::from("W");
                let nb_records = domain_records.len();
                if nb_records > 1 {
                    let power_record = &energy_records_to_power_record(
                        (
                            domain_records.get(nb_records - 1).unwrap(),
                            domain_records.get(nb_records - 2).unwrap()
                        )
                    ).unwrap();
                    power = power_record.value.clone();
                    unit = power_record.unit.to_string();    
                }
                let mut rec_dom_j_1 = String::from("unknown");
                let mut rec_dom_j_2 = String::from("unknown");
                let mut rec_dom_j_3 = String::from("unknown");
                if domain_records.len() > 2 { rec_dom_j_1 = domain_records.get(nb_records - 3).unwrap().value.to_string().trim().to_string(); }
                if domain_records.len() > 1 { rec_dom_j_2 = domain_records.get(nb_records - 2).unwrap().value.to_string().trim().to_string(); }
                if domain_records.len() > 0 { rec_dom_j_3 = domain_records.get(nb_records - 1).unwrap().value.to_string().trim().to_string(); }
                println!(
                    "socket:{} domain:{}:{} {} {} last3(uJ): {} {} {}", 
                    socket_id, domain.id, domain.name.trim(), power, unit, rec_dom_j_1, rec_dom_j_2, rec_dom_j_3
                );
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