pub mod powercap_rapl;
pub mod units;

use std::error::Error;

use std::collections::HashMap;
use std::{fmt, fs};
use std::time::{SystemTime, Duration};

use std::mem::size_of_val;


pub trait RecordGenerator{
    fn refresh_record(&mut self) -> Record;

    fn get_records_passive(&self) -> Vec<Record>;
}

pub fn energy_records_to_power_record(
    measures: (&Record, &Record)) -> Result<Record, String>
{
    let joules_1 = units::Unit::to(
        measures.0.value.trim().parse().unwrap(), &measures.0.unit, &units::Unit::Joule
    );
    let joules_2 = units::Unit::to(
        measures.1.value.trim().parse().unwrap(), &measures.1.unit, &units::Unit::Joule
    );
    let joules = joules_1.unwrap() - joules_2.unwrap();

    let t1 = measures.0.timestamp.as_secs();
    let t2 = measures.1.timestamp.as_secs();
    let time_diff =  t1 - t2;
    let result = joules / time_diff as f64; 
    Ok(Record::new(measures.1.timestamp.clone(), result.to_string(), units::Unit::Watt))
}

// !!!!!!!!!!!!!!!!! Topology !!!!!!!!!!!!!!!!!!!!!!!
/// Topology struct represents the whole CPUSocket architecture,
/// from the electricity consumption point of view,
/// including the potentially multiple CPUSocket sockets.
/// Owns a vector of CPUSocket structs representing each socket.
#[derive(Debug)]
pub struct Topology {
    pub sockets: Vec<CPUSocket>,
    pub remote: bool,
    pub cpuinfo: String,
}

impl Topology {
    pub fn new() -> Topology {
        Topology {
            sockets: vec![],
            cpuinfo: Topology::get_attributes_linux(),
            remote: false
        }
    }

    pub fn get_attributes_linux() -> String{
        let f = fs::read_to_string("/proc/cpuinfo");
        f.unwrap()
    }

    pub fn safe_add_socket(
        &mut self, socket_id: u16, domains: Vec<Domain>,
        attributes: Vec<Vec<HashMap<String, String>>>,
        counter_uj_path: String, buffer_max_kbytes: u16
    ) {
        let result: Vec<&CPUSocket> = self.sockets.iter().filter(|s| s.id == socket_id).collect();
        if result.len() == 0 {
            let socket = CPUSocket::new(
                socket_id, domains, attributes, counter_uj_path, buffer_max_kbytes
            );
            self.sockets.push(socket);
        }
    }

    pub fn get_sockets(&mut self) -> &mut Vec<CPUSocket> {
        let mutref = &mut self.sockets;
        mutref
    }

    pub fn safe_add_domain_to_socket(
        &mut self, socket_id: u16, domain_id: u16,
        name: &str, uj_counter: &str, buffer_max_kbytes: u16
    ) {
        let iterator = self.sockets.iter_mut();
        for socket in iterator {
            if socket.id == socket_id {
                socket.safe_add_domain(
                    Domain::new(
                        domain_id, String::from(name), String::from(uj_counter), buffer_max_kbytes
                    )
                );
            }
        }
    }
}

// !!!!!!!!!!!!!!!!! CPUSocket !!!!!!!!!!!!!!!!!!!!!!!
/// CPUSocket struct represents a CPU socket, owning CPU cores.
#[derive(Debug)]
pub struct CPUSocket {
    pub id: u16,
    pub domains: Vec<Domain>,
    pub attributes: Vec<Vec<HashMap<String, String>>>,
    pub counter_uj_path: String,
    pub record_buffer: Vec<Record>,
    pub buffer_max_kbytes: u16
}
impl RecordGenerator for CPUSocket {
    fn refresh_record(&mut self) -> Record {
        let timestamp = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH){
            Ok(n) => n,
            Err(_) => panic!("Couldn't generate timestamp")
        };
        let record = Record::new(
            timestamp,
            self.read_counter_uj().unwrap(),//.parse().unwrap(),
            units::Unit::MicroJoule
        );

        self.record_buffer.push(
            Record::new(
                record.timestamp.clone(),
                record.value.clone(),
                units::Unit::MicroJoule
            )
        );

        let record_buffer_ptr = &self.record_buffer;
        if size_of_val(record_buffer_ptr) > (self.buffer_max_kbytes*1000) as usize {
            let size_diff = size_of_val(record_buffer_ptr) - (self.buffer_max_kbytes*1000) as usize;
            println!("Cleaning socket records buffer !!!!!!!!!!!!!!!!!!!!");
            let nb_records_to_delete = size_diff % size_of_val(&self.record_buffer[0]);
            for _ in 1..nb_records_to_delete {
                if self.record_buffer.len() > 0 {
                    self.record_buffer.remove(0);
                }
            }
        }
        record
    }

    fn get_records_passive(&self) -> Vec<Record> {
        let mut result = vec![];
        for r in &self.record_buffer {
            result.push(
                Record::new(
                    r.timestamp.clone(), r.value.clone(), units::Unit::MicroJoule
                )
            );
        }
        result
    }
}
impl CPUSocket {
    fn new(
        id: u16, domains: Vec<Domain>, attributes: Vec<Vec<HashMap<String, String>>>,
        counter_uj_path: String, buffer_max_kbytes: u16
    ) -> CPUSocket {
        CPUSocket { id, domains, attributes, counter_uj_path, record_buffer: vec![], buffer_max_kbytes }
    }

    fn safe_add_domain(&mut self, domain: Domain) {
        let result: Vec<&Domain> = self.domains.iter().filter(|d| d.id == domain.id).collect();
        if result.len() == 0 {
            self.domains.push(domain);
        }
    }

    pub fn read_counter_uj(&self) -> Result<String, Box<dyn Error>> {
        match fs::read_to_string(&self.counter_uj_path) {
            Ok(result) => Ok(result),
            Err(error) => Err(Box::new(error))
        }
    }

    pub fn get_domains(&mut self) -> &mut Vec<Domain> {
        let mutref = &mut self.domains;
        mutref
    }

}

// !!!!!!!!!!!!!!!!! Domain !!!!!!!!!!!!!!!!!!!!!!!
/// Domain struct represents a part of a CPUSocket from the
/// electricity consumption point of view.
#[derive(Debug)]
pub struct Domain {
    pub id: u16,
    pub name: String,
    pub counter_uj_path: String,
    pub record_buffer: Vec<Record>,
    pub buffer_max_kbytes: u16
}
impl RecordGenerator for Domain {
    fn refresh_record(&mut self) -> Record {
        let timestamp = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH){
            Ok(n) => n,
            Err(_) => panic!("Couldn't generate timestamp")
        };
        let record = Record::new(
            timestamp,
            self.read_counter_uj().unwrap(),//.parse().unwrap(),
            units::Unit::MicroJoule
        );

        self.record_buffer.push(
            Record::new(
                record.timestamp.clone(),
                record.value.clone(),
                units::Unit::MicroJoule
            )
        );

        let record_buffer_ptr = &self.record_buffer;
        if size_of_val(record_buffer_ptr) > (self.buffer_max_kbytes*1000) as usize {
            let size_diff = size_of_val(record_buffer_ptr) - (self.buffer_max_kbytes*1000) as usize;
            println!("Cleaning record buffer !!!!!!!!!!!!!!!!!!!!!!!!!!!!");
            let nb_records_to_delete = size_diff % size_of_val(&self.record_buffer[0]);
            for _ in 1..nb_records_to_delete {
                if self.record_buffer.len() > 0 {
                    self.record_buffer.remove(0);
                }
            }
        }
        record
    }

    fn get_records_passive(&self) -> Vec<Record> {
        let mut result = vec![];
        for r in &self.record_buffer {
            result.push(
                Record::new(
                    r.timestamp.clone(), r.value.clone(), units::Unit::MicroJoule
                )
            );
        }
        result
    }
}
impl Domain {
    fn new(id: u16, name: String, counter_uj_path: String, buffer_max_kbytes: u16) -> Domain{
        Domain{ id, name, counter_uj_path, record_buffer: vec![], buffer_max_kbytes }
    }
    pub fn read_counter_uj(&self) -> Result<String, Box<dyn Error>> {
        match fs::read_to_string(&self.counter_uj_path) {
            Ok(result) => Ok(result),
            Err(error) => Err(Box::new(error))
        }
    }
}
impl fmt::Display for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Domain: {}", self.name)
    }
}


// !!!!!!!!!!!!!!!!! Record !!!!!!!!!!!!!!!!!!!!!!!
/// Record struct represents an electricity consumption measurement
/// tied to a domain.
#[derive(Debug)]
pub struct Record{
    pub timestamp: Duration,
    pub value: String,
    pub unit: units::Unit
}

impl Record {
    fn new(timestamp: Duration, value: String, unit: units::Unit) -> Record{
        Record{ timestamp, value, unit }
    }
}


impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "recorded {} {} at {:?}", self.value.trim(), self.unit, self.timestamp)
    }
}

// !!!!!!!!!!!!!!!!! Sensor !!!!!!!!!!!!!!!!!!!!!!!
/// Sensor trait, the Sensor API.
pub trait Sensor {
    fn get_topology(&mut self) -> Box<Option<Topology>>;
    fn generate_topology (&self) -> Result<Topology, Box<dyn Error>>;
}
