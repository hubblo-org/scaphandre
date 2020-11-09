pub mod powercap_rapl;
pub mod units;
mod utils;

use std::error::Error;

use std::collections::HashMap;
use std::{fmt, fs};
use std::time::{SystemTime, Duration};
use std::io::{self, BufReader, BufRead};
use std::fs::File;
use regex::Regex;

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
}

impl Topology {
    pub fn new() -> Topology {
        Topology {
            sockets: vec![],
            remote: false
        }
    }

    /// Parses /proc/cpuinfo and creates instances of CPUCore.
    /// 
    ///# Examples
    ///
    /// ```
    /// use scaphandre::sensors::Topology;
    /// 
    /// let cores = Topology::generate_cpu_cores().unwrap();
    /// println!("There are {} cores on this host.", cores.len());
    /// for c in &cores {
    ///     println!("Here is CPU Core number {}", c.attributes.get("processor").unwrap());
    /// }
    /// ```
    pub fn generate_cpu_cores() -> Result<Vec<CPUCore>, io::Error>{
        let f = File::open("/proc/cpuinfo")?;
        let reader = BufReader::new(f);
        let mut cores = vec![];
        let mut map = HashMap::new();
        let mut counter = 0;
        for line in reader.lines() {
            let parts = line.unwrap().trim().split(":").map(
                |x| String::from(x)
            ).collect::<Vec<String>>();
            if parts.len() >= 2 {
                let key = parts[0].trim();
                let value = parts[1].trim();
                if key == "processor" {
                    if counter > 0 {
                        cores.push(CPUCore::new(counter, map));
                        map = HashMap::new();
                    }
                    counter += 1;
                }
                map.insert(String::from(key), String::from(value));
            }
        }
        cores.push(CPUCore::new(counter, map));
        Ok(cores)
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

    pub fn add_cpu_cores(&mut self) {
        let mut cores = Topology::generate_cpu_cores().unwrap();
        while cores.len() > 0 {
            let c = cores.pop().unwrap();
            let socket_id = &c.attributes.get("physical id").unwrap().parse::<u16>().unwrap();
            let socket = self.sockets.iter_mut().filter(
                |x| &x.id == socket_id
            ).next().unwrap();
            //for s in self.sockets.iter_mut() {
            if socket_id == &socket.id {
                socket.add_cpu_core(c);
            }
            //}
        } 
        //for mut s in self.sockets.iter_mut() {
        //    let result = c.into_iter().filter(
        //        |x| x.attributes.get("physicalid").unwrap().parse::<u16>().unwrap() == s.id
        //    ).next().unwrap();
        //    s.add_cpu_core(result);
        //}
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
    pub buffer_max_kbytes: u16,
    pub cpu_cores: Vec<CPUCore>
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
    /// Simple creation of a CPUSocket instance with an empty buffer and no CPUCore owned yet.
    fn new(
        id: u16, domains: Vec<Domain>, attributes: Vec<Vec<HashMap<String, String>>>,
        counter_uj_path: String, buffer_max_kbytes: u16
    ) -> CPUSocket {
        CPUSocket {
            id, domains, attributes, counter_uj_path,
            record_buffer: vec![], // buffer has to be empty first
            buffer_max_kbytes,
            cpu_cores: vec![] // cores are instantiated on a later step
        }
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

    pub fn add_cpu_core(&mut self, core: CPUCore) {
        self.cpu_cores.push(core);
    }

    /// Returns an array of Record instances containing a number of jiffries
    /// (time metrics relative to cpu vendor) elapsed. 
    /// 
    /// 1st column : user = normal processes executing in user mode
    /// 2nd column : nice = niced processes executing in user mode
    /// 3rd column : system = processes executing in kernel mode
    /// 4th column : idle = twiddling thumbs
    /// 5th column : iowait = waiting for I/O to complete
    /// 6th column : irq = servicing interrupts
    /// 7th column : softirq = servicing softirqs
    pub fn get_usage_jiffries(&self) -> Result<[Record; 3], String> {
        let f = File::open("/proc/stat").unwrap();
        //let reader = BufReader(f);
        let reader = BufReader::new(f);
        let re_str = "cpu .*";
        let re = Regex::new(&re_str).unwrap();
        for line in reader.lines() {
            let res =  line.unwrap();
            if re.is_match(&res) {
                let parts = &res.split(" ").map(
                    |x| String::from(x)
                ).collect::<Vec<String>>();
                return Ok(
                    [
                        utils::create_record_from_jiffries(parts[1].clone()),
                        utils::create_record_from_jiffries(parts[2].clone()),
                        utils::create_record_from_jiffries(parts[3].clone())
                    ]
                )
            }
        }
        Err(String::from("Could'nt generate records for cpu core."))
    }

}

// !!!!!!!!!!!!!!!!! CPUCore !!!!!!!!!!!!!!!!!!!!!!!
/// CPUCore reprensents each CPU core on the host,
/// owned by a CPUSocket. CPUCores are instanciated regardless if
/// HyperThreading is activated on the host.Topology
#[derive(Debug)]
pub struct CPUCore {
    pub id: u16,
    pub attributes: HashMap<String, String>,
}

impl CPUCore {
    pub fn new(id: u16, attributes: HashMap<String, String>) -> CPUCore{
        CPUCore { id, attributes }
    }
    
    /// Returns an array of Record instances containing a number of jiffries
    /// (time metrics relative to cpu vendor) elapsed. 
    /// 
    /// 1st column : user = normal processes executing in user mode
    /// 2nd column : nice = niced processes executing in user mode
    /// 3rd column : system = processes executing in kernel mode
    /// 4th column : idle = twiddling thumbs
    /// 5th column : iowait = waiting for I/O to complete
    /// 6th column : irq = servicing interrupts
    /// 7th column : softirq = servicing softirqs
    pub fn get_usage_jiffries(&self) -> Result<[Record; 3], String> {
        let f = File::open("/proc/stat").unwrap();
        //let reader = BufReader(f);
        let reader = BufReader::new(f);
        let re_str = format!("cpu{} .*", self.id);
        let re = Regex::new(&re_str).unwrap();
        for line in reader.lines() {
            let res =  line.unwrap();
            if re.is_match(&res) {
                let parts = &res.split(" ").map(
                    |x| String::from(x)
                ).collect::<Vec<String>>();
                return Ok(
                    [
                        utils::create_record_from_jiffries(parts[1].clone()),
                        utils::create_record_from_jiffries(parts[2].clone()),
                        utils::create_record_from_jiffries(parts[3].clone())
                    ]
                )
            }
        }
        Err(String::from("Could'nt generate records for cpu core."))
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


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn get_proc_cpuinfo() {
        let cores = Topology::generate_cpu_cores().unwrap();
        println!("cores: {} attributes in core 0: {}", cores.len(), cores[0].attributes.len());
        for c in &cores {
            println!("{:?}", c.attributes.get("processor"));
        }
        assert_eq!(cores.len() > 0, true);
        for c in &cores {
            assert_eq!(c.attributes.len() > 5, true);
        }
    }
}