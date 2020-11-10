use std::time::{Instant, Duration};
use std::thread;
use std::collections::HashMap;
use crate::exporters::{Exporter, ExporterOption};
use crate::sensors::{Sensor, Record, Topology, RecordGenerator, energy_records_to_power_record};


pub struct StdoutExporter {
    timeout: String,
    topology: Topology
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
    pub fn new(mut sensor: Box<dyn Sensor>, timeout: String) -> StdoutExporter {
        let some_topology = *sensor.get_topology();
        StdoutExporter { timeout, topology: some_topology.unwrap() }    
    }

    pub fn runner (&mut self) {
        if self.timeout.len() == 0 {
            self.iteration(0);
        } else {
            let now = Instant::now();

            let timeout_secs: u64 = self.timeout.parse().unwrap();
            let step = 1;

            println!("Measurement step is: {}s", step);

            while now.elapsed().as_secs() <= timeout_secs {
                self.iteration(step);
                thread::sleep(Duration::new(step, 0)); 
            }
        }
    }
    fn iteration(&mut self, _step: u64){
        self.topology.refresh();
        for socket in self.topology.get_sockets() {
            let socket_id = socket.id;
            let socket_records = socket.get_records_passive();
            let mut power = String::from("unknown");
            let mut unit = String::from("W");
            let nb_records = socket_records.len();
            if nb_records > 1 {
                let power_record = &energy_records_to_power_record(
                   (
                       socket_records.get(nb_records - 1).unwrap(),
                       socket_records.get(nb_records - 2).unwrap()
                   )
                ).unwrap();
                power = power_record.value.clone();
                unit = power_record.unit.to_string();
            }
            let mut rec_j_1 = String::from("unknown");
            let mut rec_j_2 = String::from("unknown");
            let mut rec_j_3 = String::from("unknown");
            if socket_records.len() > 2 { rec_j_1 = socket_records.get(nb_records - 3).unwrap().value.to_string().trim().to_string(); }
            if socket_records.len() > 1 { rec_j_2 = socket_records.get(nb_records - 2).unwrap().value.to_string().trim().to_string(); }
            if socket_records.len() > 0 { rec_j_3 = socket_records.get(nb_records - 1).unwrap().value.to_string().trim().to_string(); }
            println!(
                "socket:{} {} {} last3(uJ): {} {} {}",
                socket_id, power, unit, rec_j_1, rec_j_2, rec_j_3
            );
            //let jiffies = socket.get_usage_jiffies().unwrap();
            //println!(
            //    "user process jiffies: {}\n |
            //    niced process jiffies: {}\n |
            //    system process jiffies: {}\n",
            //    jiffies[0].value,
            //    jiffies[1].value,
            //    jiffies[2].value,
            //);

            //let total_jiffies=
            //    jiffies[0].value.parse::<u64>().unwrap()
            //    + jiffies[1].value.parse::<u64>().unwrap()
            //    + jiffies[2].value.parse::<u64>().unwrap();

            for domain in socket.get_domains() {
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
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn get_cons_socket0(){

    }
}