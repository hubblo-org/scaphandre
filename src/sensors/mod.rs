pub mod powercap_rapl;
pub mod units;

use std::error::Error;

use std::collections::HashMap;
use std::{fmt, fs};
use std::time::{SystemTime, Duration};

use std::mem::size_of_val;


pub trait RecordGenerator{
    fn get_record(&mut self) -> Record;
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
        counter_uj_path: String, buffer_max_kB: u16
    ) {
        let result: Vec<&CPUSocket> = self.sockets.iter().filter(|s| s.id == socket_id).collect();
        if result.len() == 0 {
            let socket = CPUSocket::new(
                socket_id, domains, attributes, counter_uj_path, buffer_max_kB
            );
            self.sockets.push(socket);
        }
    }

    pub fn safe_add_domain_to_socket(
        &mut self, socket_id: u16, domain_id: u16,
        name: &str, uj_counter: &str, buffer_max_kB: u16
    ) {
        let iterator = self.sockets.iter_mut();
        for socket in iterator {
            if socket.id == socket_id {
                socket.safe_add_domain(
                    Domain::new(
                        domain_id, String::from(name), String::from(uj_counter), buffer_max_kB
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
    pub buffer_max_kB: u16
}
impl RecordGenerator for CPUSocket {
    fn get_record(&mut self) -> Record {
        let timestamp = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH){
            Ok(n) => n,
            Err(_) => panic!("Couldn't generate timestamp")
        };
        println!("counter_uj DEBUG: {}", self.read_counter_uj().unwrap());
        Record::new(
            timestamp,
            self.read_counter_uj().unwrap(),//.parse().unwrap(),
            units::EnergyUnit::MicroJoule
        )
    }
}
impl CPUSocket {
    fn new(
        id: u16, domains: Vec<Domain>, attributes: Vec<Vec<HashMap<String, String>>>,
        counter_uj_path: String, buffer_max_kB: u16
    ) -> CPUSocket {
        CPUSocket { id, domains, attributes, counter_uj_path, record_buffer: vec![], buffer_max_kB }
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
    pub buffer_max_kB: u16
}
impl RecordGenerator for Domain {
    fn get_record(&mut self) -> Record {
        let timestamp = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH){
            Ok(n) => n,
            Err(_) => panic!("Couldn't generate timestamp")
        };
        let record = Record::new(
            timestamp,
            self.read_counter_uj().unwrap(),//.parse().unwrap(),
            units::EnergyUnit::MicroJoule
        );

        self.record_buffer.push(
            Record::new(
                record.timestamp.clone(),
                record.value.clone(),
                units::EnergyUnit::MicroJoule
            )
        );

        let record_buffer_ptr = &self.record_buffer;
        if size_of_val(record_buffer_ptr) >= self.buffer_max_kB as usize {

        }
        record
    }
}
impl Domain {
    fn new(id: u16, name: String, counter_uj_path: String, buffer_max_kB: u16) -> Domain{
        Domain{ id, name, counter_uj_path, record_buffer: vec![], buffer_max_kB }
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
    pub unit: units::EnergyUnit
}

impl Record {
    fn new(timestamp: Duration, value: String, unit: units::EnergyUnit) -> Record{
        Record{ timestamp, value, unit }
    }
}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "recorded {} {} at {:?}", self.value, self.unit, self.timestamp)
    }
}

// !!!!!!!!!!!!!!!!! Sensor !!!!!!!!!!!!!!!!!!!!!!!
/// Sensor trait, the Sensor API.
pub trait Sensor {
    fn get_topology(&self) -> Box<Option<Topology>>;
    fn generate_topology (&self) -> Result<Topology, Box<dyn Error>>;
}
