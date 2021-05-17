pub mod powercap_rapl;
pub mod units;
pub mod utils;
use procfs::{process, CpuInfo, CpuTime, KernelStats};
use std::collections::HashMap;
use std::error::Error;
use std::mem::size_of_val;
use std::time::{Duration, SystemTime};
use std::{fmt, fs};
use utils::{current_system_time_since_epoch, ProcessTracker};

// !!!!!!!!!!!!!!!!! Sensor !!!!!!!!!!!!!!!!!!!!!!!
/// Sensor trait, the Sensor API.
pub trait Sensor {
    fn get_topology(&mut self) -> Box<Option<Topology>>;
    fn generate_topology(&self) -> Result<Topology, Box<dyn Error>>;
}

/// Defines methods for Record instances creation
/// and storage.
pub trait RecordGenerator {
    fn refresh_record(&mut self) -> Record;
    fn get_records_passive(&self) -> Vec<Record>;
    fn clean_old_records(&mut self);
}

// !!!!!!!!!!!!!!!!! Topology !!!!!!!!!!!!!!!!!!!!!!!
/// Topology struct represents the whole CPUSocket architecture,
/// from the electricity consumption point of view,
/// including the potentially multiple CPUSocket sockets.
/// Owns a vector of CPUSocket structs representing each socket.
#[derive(Debug, Clone)]
pub struct Topology {
    /// The CPU sockets found on the host, represented as CPUSocket instances attached to this topology
    pub sockets: Vec<CPUSocket>,
    /// ProcessTrack instance that keeps track of processes running on the host and CPU stats associated
    pub proc_tracker: ProcessTracker,
    /// CPU usage stats buffer
    pub stat_buffer: Vec<CPUStat>,
    /// Measurements of energy usage, stored as Record instances
    pub record_buffer: Vec<Record>,
    /// Maximum size in memory for the recor_buffer
    pub buffer_max_kbytes: u16,
    /// Sorted list of all domains names
    pub domains_names: Option<Vec<String>>,
}

impl RecordGenerator for Topology {
    /// Computes a new Record, stores it in the record_buffer
    /// and returns a clone of this record
    fn refresh_record(&mut self) -> Record {
        let mut value: u64 = 0;
        for s in self.get_sockets() {
            let records = s.get_records_passive();
            if !records.is_empty() {
                value += records.last().unwrap().value.trim().parse::<u64>().unwrap();
            }
        }
        let timestamp = current_system_time_since_epoch();
        let record = Record::new(timestamp, value.to_string(), units::Unit::MicroJoule);

        self.record_buffer.push(Record::new(
            record.timestamp,
            record.value.clone(),
            units::Unit::MicroJoule,
        ));

        if !self.record_buffer.is_empty() {
            self.clean_old_records();
        }
        record
    }

    /// Removes (and thus drops) as many Record instances from the record_buffer
    /// as needed for record_buffer to not exceed 'buffer_max_kbytes'
    fn clean_old_records(&mut self) {
        let record_ptr = &self.record_buffer[0];
        let record_size = size_of_val(record_ptr);
        let curr_size = record_size * self.record_buffer.len();
        trace!(
            "topology: current size of record buffer: {} max size: {}",
            curr_size,
            self.buffer_max_kbytes * 1000
        );
        if curr_size as u16 > self.buffer_max_kbytes * 1000 {
            let size_diff = curr_size - (self.buffer_max_kbytes * 1000) as usize;
            trace!(
                "topology: size_diff: {} record size: {}",
                size_diff,
                record_size
            );
            if size_diff > record_size {
                let nb_records_to_delete = size_diff as f32 / record_size as f32;
                for _ in 1..nb_records_to_delete as u32 {
                    if !self.record_buffer.is_empty() {
                        let res = self.record_buffer.remove(0);
                        debug!("Cleaning record buffer on Topology, removing: {:?}", res);
                    }
                }
            }
        }
    }

    /// Returns a copy of the record_buffer
    fn get_records_passive(&self) -> Vec<Record> {
        let mut result = vec![];
        for r in &self.record_buffer {
            result.push(Record::new(
                r.timestamp,
                r.value.clone(),
                units::Unit::MicroJoule,
            ));
        }
        result
    }
}

impl Default for Topology {
    fn default() -> Self {
        Self::new()
    }
}

impl Topology {
    /// Instanciates Topology and returns the instance
    pub fn new() -> Topology {
        Topology {
            sockets: vec![],
            proc_tracker: ProcessTracker::new(5),
            stat_buffer: vec![],
            record_buffer: vec![],
            buffer_max_kbytes: 1,
            domains_names: None,
        }
    }

    /// Parses /proc/cpuinfo and creates instances of CPUCore.
    ///
    ///# Examples
    ///
    /// ```
    /// use scaphandre::sensors::Topology;
    ///
    /// if let Ok(cores) = Topology::generate_cpu_cores() {
    ///     println!("There are {} cores on this host.", cores.len());
    ///     for c in &cores {
    ///         println!("Here is CPU Core number {}", c.attributes.get("processor").unwrap());
    ///     }
    /// }
    /// ```
    pub fn generate_cpu_cores() -> Result<Vec<CPUCore>, String> {
        let cpuinfo = CpuInfo::new().unwrap();
        let mut cores = vec![];
        for id in 0..(cpuinfo.num_cores() - 1) {
            let mut info = HashMap::new();
            for (k, v) in cpuinfo.get_info(id).unwrap().iter() {
                info.insert(String::from(*k), String::from(*v));
            }
            cores.push(CPUCore::new(id as u16, info));
        }
        Ok(cores)
    }

    /// Adds a Socket instance to self.sockets if and only if the
    /// socket id doesn't exist already.
    pub fn safe_add_socket(
        &mut self,
        socket_id: u16,
        domains: Vec<Domain>,
        attributes: Vec<Vec<HashMap<String, String>>>,
        counter_uj_path: String,
        buffer_max_kbytes: u16,
    ) {
        let result: Vec<&CPUSocket> = self.sockets.iter().filter(|s| s.id == socket_id).collect();
        if result.is_empty() {
            let socket = CPUSocket::new(
                socket_id,
                domains,
                attributes,
                counter_uj_path,
                buffer_max_kbytes,
            );
            self.sockets.push(socket);
        }
    }

    /// Returns a immutable reference to self.proc_tracker
    pub fn get_proc_tracker(&self) -> &ProcessTracker {
        &self.proc_tracker
    }

    /// Returns a mutable reference to self.sockets
    pub fn get_sockets(&mut self) -> &mut Vec<CPUSocket> {
        &mut self.sockets
    }

    /// Returns an immutable reference to self.sockets
    pub fn get_sockets_passive(&self) -> &Vec<CPUSocket> {
        &self.sockets
    }

    // Build a sorted list of all domains names from all sockets.
    fn build_domains_names(&mut self) {
        let mut names: HashMap<String, ()> = HashMap::new();
        for s in self.sockets.iter() {
            for d in s.get_domains_passive() {
                names.insert(d.name.clone(), ());
            }
        }
        let mut domain_names = names.keys().cloned().collect::<Vec<String>>();
        domain_names.sort();
        self.domains_names = Some(domain_names);
    }

    /// Adds a Domain instance to a given socket, if and only if the domain
    /// id doesn't exist already for the socket.
    pub fn safe_add_domain_to_socket(
        &mut self,
        socket_id: u16,
        domain_id: u16,
        name: &str,
        uj_counter: &str,
        buffer_max_kbytes: u16,
    ) {
        let iterator = self.sockets.iter_mut();
        for socket in iterator {
            if socket.id == socket_id {
                socket.safe_add_domain(Domain::new(
                    domain_id,
                    String::from(name),
                    String::from(uj_counter),
                    buffer_max_kbytes,
                ));
            }
        }
        self.build_domains_names();
    }

    /// Generates CPUCore instances for the host and adds them
    /// to appropriate CPUSocket instance from self.sockets
    pub fn add_cpu_cores(&mut self) {
        let mut cores = Topology::generate_cpu_cores().unwrap();
        while !cores.is_empty() {
            let c = cores.pop().unwrap();
            let socket_id = &c
                .attributes
                .get("physical id")
                .unwrap()
                .parse::<u16>()
                .unwrap();
            let socket = self
                .sockets
                .iter_mut()
                .find(|x| &x.id == socket_id)
                .expect("Trick: if you are running on a vm, do not forget to use --vm parameter invoking scaphandre at the command line");
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
        self.refresh_procs();
        self.refresh_record();
        self.refresh_stats();
    }

    /// Gets currently running processes (as procfs::Process instances) and stores
    /// them in self.proc_tracker
    fn refresh_procs(&mut self) {
        //! current_procs is the up to date list of processus running on the host
        let current_procs = process::all_processes().unwrap();

        for p in current_procs {
            let pid = p.pid;
            let res = self.proc_tracker.add_process_record(p);
            match res {
                Ok(_) => {}
                Err(msg) => panic!("Failed to track process with pid {} !\nGot: {}", pid, msg),
            }
        }
    }

    /// Gets currents stats and stores them as a CPUStat instance in self.stat_buffer
    pub fn refresh_stats(&mut self) {
        self.stat_buffer.insert(0, self.read_stats().unwrap());
        if !self.stat_buffer.is_empty() {
            self.clean_old_stats();
        }
    }

    /// Checks the size in memory of stats_buffer and deletes as many CPUStat
    /// instances from the buffer to make it smaller in memory than buffer_max_kbytes.
    fn clean_old_stats(&mut self) {
        let stat_ptr = &self.stat_buffer[0];
        let size_of_stat = size_of_val(stat_ptr);
        let curr_size = size_of_stat * self.stat_buffer.len();
        trace!("current_size of stats in topo: {}", curr_size);
        if curr_size > (self.buffer_max_kbytes * 1000) as usize {
            let size_diff = curr_size - (self.buffer_max_kbytes * 1000) as usize;
            if size_diff > size_of_stat {
                let nb_stats_to_delete = size_diff as f32 / size_of_stat as f32;
                trace!(
                    "nb_stats_to_delete: {} size_diff: {} size of: {}",
                    nb_stats_to_delete,
                    size_diff,
                    size_of_stat
                );
                for _ in 1..nb_stats_to_delete as u32 {
                    if !self.stat_buffer.is_empty() {
                        let res = self.stat_buffer.pop();
                        debug!("Cleaning topology stat buffer, removing: {:?}", res);
                    }
                }
            }
        }
    }

    /// Returns a Record instance containing the difference (attribute by attribute, except timestamp which will be the timestamp from the last record)
    /// between the last (in time) record from self.record_buffer and the previous one
    pub fn get_records_diff(&self) -> Option<Record> {
        let len = self.record_buffer.len();
        if len > 2 {
            let last = self.record_buffer.last().unwrap();
            let previous = self.record_buffer.get(len - 2).unwrap();
            let last_value = last.value.parse::<u64>().unwrap();
            let previous_value = previous.value.parse::<u64>().unwrap();
            if previous_value <= last_value {
                let diff = last_value - previous_value;
                return Some(Record::new(last.timestamp, diff.to_string(), last.unit));
            }
        }
        None
    }

    /// Returns a Record instance containing the power consumed between
    /// last and previous measurement, in microwatts.
    pub fn get_records_diff_power_microwatts(&self) -> Option<Record> {
        if self.record_buffer.len() > 1 {
            let last_record = self.record_buffer.last().unwrap();
            let previous_record = self
                .record_buffer
                .get(self.record_buffer.len() - 2)
                .unwrap();
            let last_microjoules = last_record.value.parse::<u64>().unwrap();
            let previous_microjoules = previous_record.value.parse::<u64>().unwrap();
            if previous_microjoules > last_microjoules {
                return None;
            }
            let microjoules = last_microjoules - previous_microjoules;
            let time_diff =
                last_record.timestamp.as_secs_f64() - previous_record.timestamp.as_secs_f64();
            let microwatts = microjoules as f64 / time_diff;
            return Some(Record::new(
                last_record.timestamp,
                (microwatts as u64).to_string(),
                units::Unit::MicroWatt,
            ));
        }
        None
    }

    /// Returns a CPUStat instance containing the difference between last
    /// and previous stats measurement (from stat_buffer), attribute by attribute.
    pub fn get_stats_diff(&self) -> Option<CPUStat> {
        if self.stat_buffer.len() > 1 {
            let last = &self.stat_buffer[0].cputime;
            let previous = &self.stat_buffer[1].cputime;
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
            return Some(CPUStat {
                cputime: CpuTime {
                    user: last.user - previous.user,
                    nice: last.nice - previous.nice,
                    system: last.system - previous.system,
                    idle: last.idle - previous.idle,
                    iowait,
                    irq,
                    softirq,
                    steal,
                    guest,
                    guest_nice,
                },
            });
        }
        None
    }

    /// Reads content from /proc/stat and extracts the stats of the whole CPU topology.
    pub fn read_stats(&self) -> Option<CPUStat> {
        let kernelstats_or_not = KernelStats::new();
        if let Ok(res_cputime) = kernelstats_or_not {
            return Some(CPUStat {
                cputime: res_cputime.total,
            });
        }
        None
    }

    /// Returns the number of processes currently available
    pub fn read_nb_process_total_count(&self) -> Option<u64> {
        if let Ok(result) = KernelStats::new() {
            return Some(result.processes);
        }
        None
    }

    /// Returns the number of processes currently in a running state
    pub fn read_nb_process_running_current(&self) -> Option<u32> {
        if let Ok(result) = KernelStats::new() {
            if let Some(procs_running) = result.procs_running {
                return Some(procs_running);
            }
        }
        None
    }
    /// Returns the number of processes currently blocked waiting
    pub fn read_nb_process_blocked_current(&self) -> Option<u32> {
        if let Ok(result) = KernelStats::new() {
            if let Some(procs_running) = result.procs_running {
                return Some(procs_running);
            }
        }
        None
    }
    /// Returns the current number of context switches
    pub fn read_nb_context_switches_total_count(&self) -> Option<u64> {
        if let Ok(result) = KernelStats::new() {
            return Some(result.ctxt);
        }
        None
    }

    /// Returns the power consumed between last and previous measurement for a given process ID, in microwatts
    pub fn get_process_power_consumption_microwatts(&self, pid: i32) -> Option<u64> {
        let tracker = self.get_proc_tracker();
        if let Some(recs) = tracker.find_records(pid) {
            if recs.len() > 1 {
                let last = recs.first().unwrap();
                let previous = recs.get(1).unwrap();
                if let Some(topo_stats_diff) = self.get_stats_diff() {
                    //trace!("Topology stats measured diff: {:?}", topo_stats_diff);
                    let process_total_time =
                        last.total_time_jiffies() - previous.total_time_jiffies();
                    let topo_total_time = topo_stats_diff.total_time_jiffies()
                        * procfs::ticks_per_second().unwrap() as f32;
                    let usage_percent = process_total_time as f64 / topo_total_time as f64;
                    let topo_conso = self.get_records_diff_power_microwatts();
                    if let Some(val) = &topo_conso {
                        //trace!("topo conso: {}", val);
                        let val_f64 = val.value.parse::<f64>().unwrap();
                        //trace!("val f64: {}", val_f64);
                        let result = (val_f64 * usage_percent) as u64;
                        //trace!("result: {}", result);
                        return Some(result);
                    }
                }
            }
        } else {
            trace!("Couldn't find records for PID: {}", pid);
        }
        None
    }

    pub fn get_process_cpu_consumption_percentage(&self, pid: i32) -> Option<f64> {
        let tracker = self.get_proc_tracker();
        if let Some(recs) = tracker.find_records(pid) {
            if recs.len() > 1 {
                let last = recs.first().unwrap();
                let previous = recs.get(1).unwrap();
                if let Some(topo_stats_diff) = self.get_stats_diff() {
                    let process_total_time =
                        last.total_time_jiffies() - previous.total_time_jiffies();

                    let topo_total_time = topo_stats_diff.total_time_jiffies()
                        * procfs::ticks_per_second().unwrap() as f32;

                    let usage = process_total_time as f64 / topo_total_time as f64;

                    return Some(usage * 100.0);
                }
            }
        }
        None
    }
}

// !!!!!!!!!!!!!!!!! CPUSocket !!!!!!!!!!!!!!!!!!!!!!!
/// CPUSocket struct represents a CPU socket (matches physical_id attribute in /proc/cpuinfo),
/// owning CPU cores (processor in /proc/cpuinfo).
#[derive(Debug, Clone)]
pub struct CPUSocket {
    /// Numerical ID of the CPU socket (physical_id in /proc/cpuinfo)
    pub id: u16,
    /// RAPL domains attached to the socket
    pub domains: Vec<Domain>,
    /// Text attributes linked to that socket, found in /proc/cpuinfo
    pub attributes: Vec<Vec<HashMap<String, String>>>,
    /// Path to the file that provides the counter for energy consumed by the socket, in microjoules.
    pub counter_uj_path: String,
    /// Comsumption records measured and stored by scaphandre for this socket.
    pub record_buffer: Vec<Record>,
    /// Maximum size of the record_buffer in kilobytes.
    pub buffer_max_kbytes: u16,
    /// CPU cores (core_id in /proc/cpuinfo) attached to the socket.
    pub cpu_cores: Vec<CPUCore>,
    /// Usage statistics records stored for this socket.
    pub stat_buffer: Vec<CPUStat>,
}

impl RecordGenerator for CPUSocket {
    /// Generates a new record of the socket energy consumption and stores it in the record_buffer.
    /// Returns a clone of this Record instance.
    fn refresh_record(&mut self) -> Record {
        let timestamp = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            Ok(n) => n,
            Err(_) => panic!("Couldn't generate timestamp"),
        };
        let raw_uj = self.read_counter_uj();
        let record = Record::new(timestamp, raw_uj.unwrap(), units::Unit::MicroJoule);

        self.record_buffer.push(Record::new(
            record.timestamp,
            record.value.clone(),
            units::Unit::MicroJoule,
        ));

        if !self.record_buffer.is_empty() {
            self.clean_old_records();
        }
        record
    }

    /// Checks the size in memory of record_buffer and deletes as many Record
    /// instances from the buffer to make it smaller in memory than buffer_max_kbytes.
    fn clean_old_records(&mut self) {
        let record_ptr = &self.record_buffer[0];
        let curr_size = size_of_val(record_ptr) * self.record_buffer.len();
        trace!(
            "socket rebord buffer current size: {} max_bytes: {}",
            curr_size,
            self.buffer_max_kbytes * 1000
        );
        if curr_size > (self.buffer_max_kbytes * 1000) as usize {
            let size_diff = curr_size - (self.buffer_max_kbytes * 1000) as usize;
            trace!(
                "socket record size_diff: {} sizeof: {}",
                size_diff,
                size_of_val(record_ptr)
            );
            if size_diff > size_of_val(record_ptr) {
                let nb_records_to_delete = size_diff as f32 / size_of_val(record_ptr) as f32;
                for _ in 1..nb_records_to_delete as u32 {
                    if !self.record_buffer.is_empty() {
                        let res = self.record_buffer.remove(0);
                        debug!(
                            "Cleaning socket id {} records buffer, removing: {}",
                            self.id, res
                        );
                    }
                }
            }
        }
    }

    /// Returns a new owned Vector being a clone of the current record_buffer.
    /// This does not affect the current buffer but is costly.
    fn get_records_passive(&self) -> Vec<Record> {
        let mut result = vec![];
        for r in &self.record_buffer {
            result.push(Record::new(
                r.timestamp,
                r.value.clone(),
                units::Unit::MicroJoule,
            ));
        }
        result
    }
}

impl CPUSocket {
    /// Creates and returns a CPUSocket instance with an empty buffer and no CPUCore owned yet.
    fn new(
        id: u16,
        domains: Vec<Domain>,
        attributes: Vec<Vec<HashMap<String, String>>>,
        counter_uj_path: String,
        buffer_max_kbytes: u16,
    ) -> CPUSocket {
        CPUSocket {
            id,
            domains,
            attributes,
            counter_uj_path,
            record_buffer: vec![], // buffer has to be empty first
            buffer_max_kbytes,
            cpu_cores: vec![], // cores are instantiated on a later step
            stat_buffer: vec![],
        }
    }

    /// Adds a new Domain instance to the domains vector if and only if it doesn't exist in the vector already.
    fn safe_add_domain(&mut self, domain: Domain) {
        let result: Vec<&Domain> = self.domains.iter().filter(|d| d.id == domain.id).collect();
        if result.is_empty() {
            self.domains.push(domain);
        }
    }

    /// Returns the content of the energy consumption counter file, as a String
    /// value of microjoules.
    pub fn read_counter_uj(&self) -> Result<String, Box<dyn Error>> {
        match fs::read_to_string(&self.counter_uj_path) {
            Ok(result) => Ok(result),
            Err(error) => Err(Box::new(error)),
        }
    }

    /// Returns a mutable reference to the domains vector.
    pub fn get_domains(&mut self) -> &mut Vec<Domain> {
        &mut self.domains
    }

    /// Returns a immutable reference to the domains vector.
    pub fn get_domains_passive(&self) -> &Vec<Domain> {
        &self.domains
    }

    /// Returns a mutable reference to the CPU cores vector.
    pub fn get_cores(&mut self) -> &mut Vec<CPUCore> {
        &mut self.cpu_cores
    }

    /// Returns a immutable reference to the CPU cores vector.
    pub fn get_cores_passive(&self) -> &Vec<CPUCore> {
        &self.cpu_cores
    }

    /// Adds a CPU core instance to the cores vector.
    pub fn add_cpu_core(&mut self, core: CPUCore) {
        self.cpu_cores.push(core);
    }

    /// Generates a new CPUStat object storing current usage statistics of the socket
    /// and stores it in the stat_buffer.
    pub fn refresh_stats(&mut self) {
        if !self.stat_buffer.is_empty() {
            self.clean_old_stats();
        }
        self.stat_buffer.insert(0, self.read_stats().unwrap());
    }

    /// Checks the size in memory of stats_buffer and deletes as many CPUStat
    /// instances from the buffer to make it smaller in memory than buffer_max_kbytes.
    fn clean_old_stats(&mut self) {
        let stat_ptr = &self.stat_buffer[0];
        let size_of_stat = size_of_val(stat_ptr);
        let curr_size = size_of_stat * self.stat_buffer.len();
        trace!("current_size of stats in socket {}: {}", self.id, curr_size);
        trace!(
            "estimated max nb of socket stats: {}",
            self.buffer_max_kbytes as f32 * 1000.0 / size_of_stat as f32
        );
        if curr_size > (self.buffer_max_kbytes * 1000) as usize {
            let size_diff = curr_size - (self.buffer_max_kbytes * 1000) as usize;
            trace!(
                "socket {} size_diff: {} size of: {}",
                self.id,
                size_diff,
                size_of_stat
            );
            if size_diff > size_of_stat {
                let nb_stats_to_delete = size_diff as f32 / size_of_stat as f32;
                trace!(
                    "socket {} nb_stats_to_delete: {} size_diff: {} size of: {}",
                    self.id,
                    nb_stats_to_delete,
                    size_diff,
                    size_of_stat
                );
                trace!("nb stats to delete: {}", nb_stats_to_delete as u32);
                for _ in 1..nb_stats_to_delete as u32 {
                    if !self.stat_buffer.is_empty() {
                        let res = self.stat_buffer.pop();
                        debug!(
                            "Cleaning stat buffer of socket {}, removing: {:?}",
                            self.id, res
                        );
                    }
                }
            }
        }
    }

    /// Combines stats from all CPU cores owned byu the socket and returns
    /// a CpuTime struct containing stats for the whole socket.
    pub fn read_stats(&self) -> Option<CPUStat> {
        let mut stats = CPUStat {
            cputime: CpuTime {
                user: 0.0,
                nice: 0.0,
                system: 0.0,
                idle: 0.0,
                iowait: Some(0.0),
                irq: Some(0.0),
                softirq: Some(0.0),
                guest: Some(0.0),
                guest_nice: Some(0.0),
                steal: Some(0.0),
            },
        };
        for c in &self.cpu_cores {
            let c_stats = c.read_stats().unwrap();
            stats.cputime.user += c_stats.user;
            stats.cputime.nice += c_stats.nice;
            stats.cputime.system += c_stats.system;
            stats.cputime.idle += c_stats.idle;
            stats.cputime.iowait =
                Some(stats.cputime.iowait.unwrap_or_default() + c_stats.iowait.unwrap_or_default());
            stats.cputime.irq =
                Some(stats.cputime.irq.unwrap_or_default() + c_stats.irq.unwrap_or_default());
            stats.cputime.softirq = Some(
                stats.cputime.softirq.unwrap_or_default() + c_stats.softirq.unwrap_or_default(),
            );
        }
        Some(stats)
    }

    /// Computes the difference between previous usage statistics record for the socket
    /// and the current one. Returns a CPUStat object containing this difference, field
    /// by field.
    pub fn get_stats_diff(&mut self) -> Option<CPUStat> {
        if self.stat_buffer.len() > 1 {
            let last = &self.stat_buffer[0].cputime;
            let previous = &self.stat_buffer[1].cputime;
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
            return Some(CPUStat {
                cputime: CpuTime {
                    user: last.user - previous.user,
                    nice: last.nice - previous.nice,
                    system: last.system - previous.system,
                    idle: last.idle - previous.idle,
                    iowait,
                    irq,
                    softirq,
                    steal,
                    guest,
                    guest_nice,
                },
            });
        }
        None
    }

    /// Returns a Record instance containing the power consumed between last
    /// and previous measurement, for this CPU socket
    pub fn get_records_diff_power_microwatts(&self) -> Option<Record> {
        if self.record_buffer.len() > 1 {
            let last_record = self.record_buffer.last().unwrap();
            let previous_record = self
                .record_buffer
                .get(self.record_buffer.len() - 2)
                .unwrap();
            debug!(
                "last_record value: {} previous_record value: {}",
                &last_record.value, &previous_record.value
            );
            if let (Ok(last_microjoules), Ok(previous_microjoules)) = (
                last_record.value.trim().parse::<u64>(),
                previous_record.value.trim().parse::<u64>(),
            ) {
                let microjoules = last_microjoules - previous_microjoules;
                let time_diff =
                    last_record.timestamp.as_secs_f64() - previous_record.timestamp.as_secs_f64();
                let microwatts = microjoules as f64 / time_diff;
                debug!("microwatts: {}", microwatts);
                return Some(Record::new(
                    last_record.timestamp,
                    (microwatts as u64).to_string(),
                    units::Unit::MicroWatt,
                ));
            }
        } else {
            debug!("Not enough records for socket");
        }
        None
    }
}

// !!!!!!!!!!!!!!!!! CPUCore !!!!!!!!!!!!!!!!!!!!!!!
/// CPUCore reprensents each CPU core on the host,
/// owned by a CPUSocket. CPUCores are instanciated regardless if
/// HyperThreading is activated on the host.
/// Reprensents the processor field in /proc/cpuinfo.
#[derive(Debug, Clone)]
pub struct CPUCore {
    pub id: u16,
    pub attributes: HashMap<String, String>,
}

impl CPUCore {
    /// Instantiates CPUCore and returns the instance.
    pub fn new(id: u16, attributes: HashMap<String, String>) -> CPUCore {
        CPUCore { id, attributes }
    }

    /// Reads content from /proc/stat and extracts the stats of the CPU core
    fn read_stats(&self) -> Option<CpuTime> {
        if let Ok(mut kernelstats) = KernelStats::new() {
            return Some(kernelstats.cpu_time.remove(self.id as usize));
        }
        None
    }
}

// !!!!!!!!!!!!!!!!! Domain !!!!!!!!!!!!!!!!!!!!!!!
/// Domain struct represents a part of a CPUSocket from the
/// electricity consumption point of view.
#[derive(Debug, Clone)]
pub struct Domain {
    /// Numerical ID of the RAPL domain as indicated in /sys/class/powercap/intel-rapl* folders names
    pub id: u16,
    /// Name of the domain as found in /sys/class/powercap/intel-rapl:X:X/name
    pub name: String,
    /// Path to the domain's energy counter file, microjoules extracted
    pub counter_uj_path: String,
    /// History of energy consumption measurements, stored as Record instances
    pub record_buffer: Vec<Record>,
    /// Maximum size of record_buffer, in kilobytes
    pub buffer_max_kbytes: u16,
}
impl RecordGenerator for Domain {
    /// Computes a measurement of energy comsumption for this CPU domain,
    /// stores a copy in self.record_buffer and returns it.
    fn refresh_record(&mut self) -> Record {
        let timestamp = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            Ok(n) => n,
            Err(_) => panic!("Couldn't generate timestamp"),
        };
        let record = Record::new(
            timestamp,
            self.read_counter_uj().unwrap(), //.parse().unwrap(),
            units::Unit::MicroJoule,
        );

        self.record_buffer.push(Record::new(
            record.timestamp,
            record.value.clone(),
            units::Unit::MicroJoule,
        ));

        if !self.record_buffer.is_empty() {
            self.clean_old_records();
        }
        record
    }

    /// Removes as many Record instances from self.record_buffer as needed
    /// for record_buffer to take less than 'buffer_max_kbytes' in memory
    fn clean_old_records(&mut self) {
        let record_ptr = &self.record_buffer[0];
        let curr_size = size_of_val(record_ptr) * self.record_buffer.len();
        if curr_size > (self.buffer_max_kbytes * 1000) as usize {
            let size_diff = curr_size - (self.buffer_max_kbytes * 1000) as usize;
            if size_diff > size_of_val(&self.record_buffer[0]) {
                let nb_records_to_delete =
                    size_diff as f32 / size_of_val(&self.record_buffer[0]) as f32;
                for _ in 1..nb_records_to_delete as u32 {
                    if !self.record_buffer.is_empty() {
                        self.record_buffer.remove(0);
                    }
                }
            }
        }
    }

    /// Returns a copy of self.record_buffer
    fn get_records_passive(&self) -> Vec<Record> {
        let mut result = vec![];
        for r in &self.record_buffer {
            result.push(Record::new(
                r.timestamp,
                r.value.clone(),
                units::Unit::MicroJoule,
            ));
        }
        result
    }
}
impl Domain {
    /// Instanciates Domain and returns the instance
    fn new(id: u16, name: String, counter_uj_path: String, buffer_max_kbytes: u16) -> Domain {
        Domain {
            id,
            name,
            counter_uj_path,
            record_buffer: vec![],
            buffer_max_kbytes,
        }
    }
    /// Reads content of this domain's energy_uj file
    pub fn read_counter_uj(&self) -> Result<String, Box<dyn Error>> {
        match fs::read_to_string(&self.counter_uj_path) {
            Ok(result) => Ok(result),
            Err(error) => Err(Box::new(error)),
        }
    }

    /// Returns a Record instance containing the power consumed between
    /// last and previous measurement, in microwatts.
    pub fn get_records_diff_power_microwatts(&self) -> Option<Record> {
        if self.record_buffer.len() > 1 {
            let last_record = self.record_buffer.last().unwrap();
            let previous_record = self
                .record_buffer
                .get(self.record_buffer.len() - 2)
                .unwrap();
            if let (Ok(last_microjoules), Ok(previous_microjoules)) = (
                last_record.value.trim().parse::<u64>(),
                previous_record.value.trim().parse::<u64>(),
            ) {
                if previous_microjoules > last_microjoules {
                    return None;
                }
                let microjoules = last_microjoules - previous_microjoules;
                let time_diff =
                    last_record.timestamp.as_secs_f64() - previous_record.timestamp.as_secs_f64();
                let microwatts = microjoules as f64 / time_diff;
                return Some(Record::new(
                    last_record.timestamp,
                    (microwatts as u64).to_string(),
                    units::Unit::MicroWatt,
                ));
            }
        }
        None
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
#[derive(Debug, Clone)]
pub struct Record {
    pub timestamp: Duration,
    pub value: String,
    pub unit: units::Unit,
}

impl Record {
    /// Instances Record and returns the instance
    pub fn new(timestamp: Duration, value: String, unit: units::Unit) -> Record {
        Record {
            timestamp,
            value,
            unit,
        }
    }
}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "recorded {} {} at {:?}",
            self.value.trim(),
            self.unit,
            self.timestamp
        )
    }
}

#[derive(Debug)]
pub struct CPUStat {
    pub cputime: CpuTime,
}

impl CPUStat {
    /// Returns the total of active CPU time spent, for this stat measurement
    /// (not iowait, idle, irq or softirq)
    pub fn total_time_jiffies(&self) -> f32 {
        let user = self.cputime.user;
        let nice = self.cputime.nice;
        let system = self.cputime.system;
        let idle = self.cputime.idle;
        let irq = self.cputime.irq.unwrap_or_default();
        let iowait = self.cputime.iowait.unwrap_or_default();
        let softirq = self.cputime.softirq.unwrap_or_default();
        let steal = self.cputime.steal.unwrap_or_default();
        let guest_nice = self.cputime.guest_nice.unwrap_or_default();
        let guest = self.cputime.guest.unwrap_or_default();

        trace!(
            "CPUStat contains user {} nice {} system {} idle: {} irq {} softirq {} iowait {} steal {} guest_nice {} guest {}",
            user, nice, system, idle, irq, softirq, iowait, steal, guest_nice, guest
        );
        user + nice + system + guest_nice + guest
    }
}

impl Clone for CPUStat {
    /// Returns a copy of CPUStat instance
    fn clone(&self) -> CPUStat {
        CPUStat {
            cputime: CpuTime {
                user: self.cputime.user,
                nice: self.cputime.nice,
                system: self.cputime.system,
                softirq: self.cputime.softirq,
                irq: self.cputime.irq,
                idle: self.cputime.idle,
                iowait: self.cputime.iowait,
                steal: self.cputime.steal,
                guest: self.cputime.guest,
                guest_nice: self.cputime.guest_nice,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn get_proc_cpuinfo() {
        let cores = Topology::generate_cpu_cores().unwrap();
        println!(
            "cores: {} attributes in core 0: {}",
            cores.len(),
            cores[0].attributes.len()
        );
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
        let mut sensor = powercap_rapl::PowercapRAPLSensor::new(8, 8, false);
        let topo = (*sensor.get_topology()).unwrap();
        println!("{:?}", topo.read_stats());
    }

    #[test]
    fn read_core_stats() {
        let mut sensor = powercap_rapl::PowercapRAPLSensor::new(8, 8, false);
        let mut topo = (*sensor.get_topology()).unwrap();
        for s in topo.get_sockets() {
            for c in s.get_cores() {
                println!("{:?}", c.read_stats());
            }
        }
    }

    #[test]
    fn read_socket_stats() {
        let mut sensor = powercap_rapl::PowercapRAPLSensor::new(8, 8, false);
        let mut topo = (*sensor.get_topology()).unwrap();
        for s in topo.get_sockets() {
            println!("{:?}", s.read_stats());
        }
    }
}

//  Copyright 2020 The scaphandre authors.
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
