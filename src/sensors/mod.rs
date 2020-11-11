pub mod powercap_rapl;
pub mod units;
mod utils;
use procfs::{process, CpuTime, KernelStats, CpuInfo};
use std::error::Error;
use std::collections::HashMap;
use std::{fmt, fs};
use std::time::{SystemTime, Duration};
use utils::{ProcessTracker, current_system_time_since_epoch};
use std::mem::size_of_val;

// !!!!!!!!!!!!!!!!! Sensor !!!!!!!!!!!!!!!!!!!!!!!
/// Sensor trait, the Sensor API.
pub trait Sensor {
    fn get_topology(&mut self) -> Box<Option<Topology>>;
    fn generate_topology (&self) -> Result<Topology, Box<dyn Error>>;
}


/// Defines methods for Record instances creation
/// and storage.
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
    Ok(Record::new(measures.1.timestamp, result.to_string(), units::Unit::Watt))
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
    pub proc_tracker: ProcessTracker,
    pub stat_buffer: Vec<CpuTime>,
    pub record_buffer: Vec<Record>,
    pub buffer_max_kbytes: u16,
}

impl RecordGenerator for Topology {
    fn refresh_record(&mut self) -> Record {
        let mut value: u64 = 0;
        for s in self.get_sockets() {
            let records = s.get_records_passive();
            if !records.is_empty() {
                value += records.get(records.len() - 1).unwrap().value.trim().parse::<u64>().unwrap();
            }
        }
        let timestamp = current_system_time_since_epoch();
        let record = Record::new(
            timestamp,
            value.to_string(),
            units::Unit::MicroJoule
        );

        self.record_buffer.push(
            Record::new(
                record.timestamp,
                record.value.clone(),
                units::Unit::MicroJoule
            )
        );

        println!("{:?}", self.record_buffer);

        let record_buffer_ptr = &self.record_buffer;
        if size_of_val(record_buffer_ptr) > (self.buffer_max_kbytes*1000) as usize {
            let size_diff = size_of_val(record_buffer_ptr) - (self.buffer_max_kbytes*1000) as usize;
            println!("Cleaning socket records buffer !!!!!!!!!!!!!!!!!!!!");
            let nb_records_to_delete = size_diff % size_of_val(&self.record_buffer[0]);
            for _ in 1..nb_records_to_delete {
                if !self.record_buffer.is_empty() {
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
                    r.timestamp, r.value.clone(), units::Unit::MicroJoule
                )
            );
        }
        result
    }
}

impl Topology {
    pub fn new() -> Topology {
        Topology {
            sockets: vec![],
            remote: false,
            proc_tracker: ProcessTracker::new(3),
            stat_buffer: vec![],
            record_buffer: vec![],
            buffer_max_kbytes: 8
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
    pub fn generate_cpu_cores() -> Result<Vec<CPUCore>, String>{
        let cpuinfo = CpuInfo::new().unwrap();
        let mut cores = vec![];
        for id in 0..(cpuinfo.num_cores()-1) {
            let mut info = HashMap::new();
            for (k, v) in cpuinfo.get_info(id).unwrap().iter() {
                info.insert(String::from(*k), String::from(*v));
            }
            cores.push(
                CPUCore::new(
                    id as u16,
                    info
                )
            );
        }
        Ok(cores) 
        //let f = File::open("/proc/cpuinfo")?;
        //let reader = BufReader::new(f);
        //let mut map = HashMap::new();
        //let mut counter = 0;
        //for line in reader.lines() {
        //    let parts = line.unwrap().trim().split(':').map(String::from).collect::<Vec<String>>();
        //    if parts.len() >= 2 {
        //        let key = parts[0].trim();
        //        let value = parts[1].trim();
        //        if key == "processor" {
        //            if counter > 0 {
        //                cores.push(CPUCore::new(value.parse::<u16>().unwrap(), map));
        //                map = HashMap::new();
        //            }
        //            counter += 1;
        //        }
        //        map.insert(String::from(key), String::from(value));
        //    }
        //}
        //cores.push(CPUCore::new(map.get("processor").unwrap().parse::<u16>().unwrap(), map));
        //Ok(cores)
    }

    pub fn safe_add_socket(
        &mut self, socket_id: u16, domains: Vec<Domain>,
        attributes: Vec<Vec<HashMap<String, String>>>,
        counter_uj_path: String, buffer_max_kbytes: u16
    ) {
        let result: Vec<&CPUSocket> = self.sockets.iter().filter(|s| s.id == socket_id).collect();
        if result.is_empty() {
            let socket = CPUSocket::new(
                socket_id, domains, attributes, counter_uj_path, buffer_max_kbytes
            );
            self.sockets.push(socket);
        }
    }

    pub fn get_sockets(&mut self) -> &mut Vec<CPUSocket> {
        &mut self.sockets
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
        while !cores.is_empty() {
            let c = cores.pop().unwrap();
            let socket_id = &c.attributes.get("physical id").unwrap().parse::<u16>().unwrap();
            let socket = self.sockets.iter_mut().find(
                |x| &x.id == socket_id
            ).unwrap();
            if socket_id == &socket.id {
                socket.add_cpu_core(c);
            }
        } 
    }

    /// Triggers ProcessTracker refresh on process stats
    /// and power consumption, CPU stats and cores power comsumption,
    /// CPU sockets stats and power consumption.
    pub fn refresh(&mut self) {
        let sockets = &mut self.sockets;
        for s in sockets {
            // refresh each socket with new record
            s.refresh_record();
            s.refresh_stats();
            let domains = s.get_domains();
            for d in domains {
                d.refresh_record();
            }
            //let cores = s.get_cores();
            //for c in cores {
            //    
            //}
        }
        self.refresh_record();
        self.refresh_procs();
        self.refresh_stats();
    }

    fn refresh_procs(&mut self) {
        //! current_procs is the up to date list of processus running on the host
        let current_procs = process::all_processes().unwrap();

        for p in current_procs {
            let pid = p.pid;
            let res = self.proc_tracker.add_process_record(p);
            match res {
                Ok(_) => {},
                Err(msg) => panic!("Failed to track process with pid {} !\nGot: {}", pid, msg)
            }
        }
    }

    pub fn refresh_stats(&mut self) {
        self.stat_buffer.insert(0, self.read_stats().unwrap());
    }

    pub fn get_stats_diff(&mut self) -> Option<CpuTime> {
        if self.stat_buffer.len() > 1 {
            let last = &self.stat_buffer[0];
            let previous = &self.stat_buffer[1];
            let mut iowait = None;
            let mut irq = None;
            let mut softirq = None;
            let mut steal = None;
            let mut guest = None;
            let mut guest_nice = None;
            if last.iowait.is_some() && previous.iowait.is_some() {
               iowait = Some(last.iowait.unwrap() - previous.iowait.unwrap());
            }
            if last.irq.is_some() && previous.irq.is_some() {
               irq = Some(last.irq.unwrap() - previous.irq.unwrap());
            }
            if last.softirq.is_some() && previous.softirq.is_some() {
               softirq = Some(last.softirq.unwrap() - previous.softirq.unwrap());
            }
            if last.steal.is_some() && previous.steal.is_some() {
               steal = Some(last.steal.unwrap() - previous.steal.unwrap());
            }
            if last.guest.is_some() && previous.guest.is_some() {
               guest = Some(last.guest.unwrap() - previous.guest.unwrap());
            }
            if last.guest_nice.is_some() && previous.guest_nice.is_some() {
               guest_nice = Some(last.guest_nice.unwrap() - previous.guest_nice.unwrap());
            }
            return Some(
                CpuTime {
                    user: last.user - previous.user,
                    nice: last.nice - previous.nice,
                    system: last.system - previous.system,
                    idle: last.idle - previous.idle,
                    iowait, irq, softirq, steal, guest, guest_nice
                }
            )
        }
        None
    }

    fn get_proc_conso(&self, pid: i32) -> Result<Record, String>{

        //    self.proc_tracker.get
        Err(
            String::from(
                format!("Couldn't get consumption for process {}. Maybe the history is too short.", pid)
            )
        )
    }
    /// Reads content from /proc/stat and extracts the stats of the whole CPU topology.
    pub fn read_stats(&self) -> Option<CpuTime> {
        let kernelstats_or_not = KernelStats::new();
        if kernelstats_or_not.is_ok() {
            return Some(kernelstats_or_not.unwrap().total);
        }
        //let f = File::open("/proc/stat").unwrap();
        //let reader = BufReader::new(f);
        //let re_str = "cpu .*";
        //let re = Regex::new(&re_str).unwrap();
        //for line in reader.lines() {
        //    let raw = line.unwrap();
        //    if re.is_match(&raw) {
        //        let res = &raw.split(' ').map(String::from).collect::<Vec<String>>();
        //        return Some(
        //            CpuTime {
        //                user: res[2].parse::<f32>().unwrap(),
        //                nice: res[3].parse::<f32>().unwrap(),
        //                system: res[4].parse::<f32>().unwrap(),
        //                idle: res[5].parse::<f32>().unwrap(),
        //                iowait: res[6].parse::<f32>().unwrap(),
        //                irq: res[7].parse::<f32>().unwrap(),
        //                softirq: res[8].parse::<f32>().unwrap()
        //            }
        //        )
        //    }
        //}
        None
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
    pub cpu_cores: Vec<CPUCore>,
    pub stat_buffer: Vec<CpuTime>
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
                record.timestamp,
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
                if !self.record_buffer.is_empty() {
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
                    r.timestamp, r.value.clone(), units::Unit::MicroJoule
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
            cpu_cores: vec![], // cores are instantiated on a later step
            stat_buffer: vec![]
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
        &mut self.domains
    }

    pub fn get_cores(&mut self) -> &mut Vec<CPUCore> {
        &mut self.cpu_cores
    }

    pub fn add_cpu_core(&mut self, core: CPUCore) {
        self.cpu_cores.push(core);
    }

    pub fn refresh_stats(&mut self) {
        self.stat_buffer.insert(0, self.read_stats().unwrap());
    }

    /// Combines stats from all CPU cores owned byu the socket and returns
    /// a CpuTime struct containing stats for the whole socket.
    pub fn read_stats(&self) -> Option<CpuTime> {
        let mut stats = CpuTime {
            user: 0.0, nice: 0.0, system: 0.0, idle: 0.0, iowait: Some(0.0),
            irq: Some(0.0), softirq: Some(0.0), guest: Some(0.0),
            guest_nice: Some(0.0), steal: Some(0.0)
        };
        for c in &self.cpu_cores {
            let c_stats = c.read_stats().unwrap();
            stats.user += c_stats.user;
            stats.nice += c_stats.nice;
            stats.system += c_stats.system;
            stats.idle += c_stats.idle;
            stats.iowait = Some(
                stats.iowait.unwrap_or_default() + c_stats.iowait.unwrap_or_default()
            );
            stats.irq = Some(
                stats.irq.unwrap_or_default() + c_stats.irq.unwrap_or_default()
            );
            stats.softirq = Some(
                stats.softirq.unwrap_or_default() + c_stats.softirq.unwrap_or_default()
            );
        }
        Some(stats)
    }
    pub fn get_stats_diff(&mut self) -> Option<CpuTime> {
        if self.stat_buffer.len() > 1 {
            let last = &self.stat_buffer[0];
            let previous = &self.stat_buffer[1];
            let mut iowait = None;
            let mut irq = None;
            let mut softirq = None;
            let mut steal = None;
            let mut guest = None;
            let mut guest_nice = None;
            if last.iowait.is_some() && previous.iowait.is_some() {
               iowait = Some(last.iowait.unwrap() - previous.iowait.unwrap());
            }
            if last.irq.is_some() && previous.irq.is_some() {
               irq = Some(last.irq.unwrap() - previous.irq.unwrap());
            }
            if last.softirq.is_some() && previous.softirq.is_some() {
               softirq = Some(last.softirq.unwrap() - previous.softirq.unwrap());
            }
            if last.steal.is_some() && previous.steal.is_some() {
               steal = Some(last.steal.unwrap() - previous.steal.unwrap());
            }
            if last.guest.is_some() && previous.guest.is_some() {
               guest = Some(last.guest.unwrap() - previous.guest.unwrap());
            }
            if last.guest_nice.is_some() && previous.guest_nice.is_some() {
               guest_nice = Some(last.guest_nice.unwrap() - previous.guest_nice.unwrap());
            }
            return Some(
                CpuTime {
                    user: last.user - previous.user,
                    nice: last.nice - previous.nice,
                    system: last.system - previous.system,
                    idle: last.idle - previous.idle,
                    iowait, irq, softirq, steal, guest, guest_nice
                }
            )
        }
        None        
    }
    //pub fn get_stats_diff(&self) -> Option<CpuTime> {
    //    if self.stat_buffer.len() > 1 {
    //        let last = &self.stat_buffer[0];
    //        let previous = &self.stat_buffer[1];
    //        return Some(
    //            CpuTime {
    //                user: last.user - previous.user,
    //                nice: last.nice - previous.nice,
    //                system: last.system - previous.system,
    //                idle: last.idle - previous.idle,
    //                iowait: last.iowait - previous.iowait,
    //                irq: last.irq - previous.irq,
    //                softirq: last.softirq - previous.softirq,

    //            }
    //        )
    //    }
    //    None
    //}
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
    
    /// Reads content from /proc/stat and extracts the stats of the socket
    fn read_stats(&self) -> Option<CpuTime> {
        let kernelstats_or_not = KernelStats::new();
        if kernelstats_or_not.is_ok() {
            return Some(
                kernelstats_or_not.unwrap().cpu_time.remove(
                    self.id as usize
                )
            );
        }
        //let f = File::open("/proc/stat").unwrap();
        //let reader = BufReader::new(f);
        //let re_str = format!("cpu{} .*", self.id);
        //let re = Regex::new(&re_str).unwrap();
        //for line in reader.lines() {
        //    let raw = line.unwrap();
        //    if re.is_match(&raw) {
        //        let res = &raw.split(' ').map(String::from).collect::<Vec<String>>();
        //        return Some(
        //            CpuTime {
        //                user: res[1].parse::<u64>().unwrap(),
        //                nice: res[2].parse::<u64>().unwrap(),
        //                system: res[3].parse::<u64>().unwrap(),
        //                idle: res[4].parse::<u64>().unwrap(),
        //                iowait: res[5].parse::<u64>().unwrap(),
        //                irq: res[6].parse::<u64>().unwrap(),
        //                softirq: res[7].parse::<u64>().unwrap()
        //            }
        //        )
        //    }
        //}
        None
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
                record.timestamp,
                record.value.clone(),
                units::Unit::MicroJoule
            )
        );

        let record_buffer_ptr = &self.record_buffer;
        if size_of_val(record_buffer_ptr) > (self.buffer_max_kbytes*1000) as usize {
            let size_diff = size_of_val(record_buffer_ptr) - (self.buffer_max_kbytes*1000) as usize;
            let nb_records_to_delete = size_diff % size_of_val(&self.record_buffer[0]);
            for _ in 1..nb_records_to_delete {
                if !self.record_buffer.is_empty() {
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
                    r.timestamp, r.value.clone(), units::Unit::MicroJoule
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
    pub fn new(timestamp: Duration, value: String, unit: units::Unit) -> Record{
        Record{ timestamp, value, unit }
    }
}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "recorded {} {} at {:?}", self.value.trim(), self.unit, self.timestamp)
    }
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

    #[test]
    fn read_topology_stats() {
        let mut sensor = powercap_rapl::PowercapRAPLSensor::new(8, 8);
        let topo = (*sensor.get_topology()).unwrap();
        println!("{:?}", topo.read_stats()) ;
    }

    #[test]
    fn read_core_stats() {
        let mut sensor = powercap_rapl::PowercapRAPLSensor::new(8, 8);
        let mut topo = (*sensor.get_topology()).unwrap();
        for s in topo.get_sockets() {
            for c in s.get_cores() {
                println!("{:?}", c.read_stats());
            }
        }
    }

    #[test]
    fn read_socket_stats() {
        let mut sensor = powercap_rapl::PowercapRAPLSensor::new(8, 8);
        let mut topo = (*sensor.get_topology()).unwrap();
        for s in topo.get_sockets() {
            println!("{:?}", s.read_stats());
        }
    }
}