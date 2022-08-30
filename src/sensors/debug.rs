use crate::sensors::{Sensor,Topology,Domain,Socket,Record,CPUStat,CPUCore};
use std::error::Error;
use super::{utils::{current_system_time_since_epoch}, units};

pub struct DebugSensor {
    buffer_per_socket_max_kbytes: u16,
}

impl DebugSensor {
    pub fn new(buffer_per_socket_max_kbytes: u16,) -> DebugSensor {
        DebugSensor {
            buffer_per_socket_max_kbytes
        }
    }
}

impl Sensor for DebugSensor {
    fn generate_topology(&self) -> Result<Topology, Box<dyn Error>> {
        let mut topo = Topology::new();
        topo.safe_add_socket(DebugSocket::new(1234, self.buffer_per_socket_max_kbytes));
        topo.safe_add_domain_to_socket(
            1234, 
            4321, 
            "debug domain", 
            "debug domain uj_counter", 
            self.buffer_per_socket_max_kbytes
        );
        Ok(topo)
    }

    fn get_topology(&mut self) -> Box<Option<Topology>> {
        Box::new(self.generate_topology().ok())
    }
}

#[derive(Debug, Clone)]
pub struct DebugSocket {
    /// Numerical ID of the CPU socket (physical_id in /proc/cpuinfo)
    pub id: u16,
    /// RAPL domains attached to the socket
    pub domains: Vec<Domain>,
    /// Comsumption records measured and stored by scaphandre for this socket.
    pub record_buffer: Vec<Record>,
    /// Maximum size of the record_buffer in kilobytes.
    pub buffer_max_kbytes: u16,
    /// CPU cores (core_id in /proc/cpuinfo) attached to the socket.
    pub cpu_cores: Vec<CPUCore>,
    /// Usage statistics records stored for this socket.
    pub stat_buffer: Vec<CPUStat>,
}

impl DebugSocket {
    pub fn new(id: u16, buffer_max_kbytes: u16) -> DebugSocket {
        DebugSocket {
            id,
            domains: vec![],
            record_buffer: vec![],
            buffer_max_kbytes,
            cpu_cores: vec![],
            stat_buffer: vec![]
        }
    }
}

impl Socket for DebugSocket {
    fn read_record_uj(&self) -> Result<Record, Box<dyn Error>> {
        Ok(Record::new(
            current_system_time_since_epoch(),
            String::from("7081760374"),
            units::Unit::MicroJoule,
        ))
    }

    /// Combines stats from all CPU cores owned byu the socket and returns
    /// a CpuTime struct containing stats for the whole socket.
    fn read_stats(&self) -> Option<CPUStat> {
        None
    }

    fn get_id(&self) -> u16 {
        self.id
    }

    fn get_record_buffer(&mut self) -> &mut Vec<Record> {
        &mut self.record_buffer
    }

    fn get_record_buffer_passive(&self) -> &Vec<Record> {
        &self.record_buffer
    }

    fn get_buffer_max_kbytes(&self) -> u16 {
        self.buffer_max_kbytes
    }

    /// Returns a mutable reference to the domains vector.
    fn get_domains(&mut self) -> &mut Vec<Domain> {
        &mut self.domains
    }

    /// Returns a immutable reference to the domains vector.
    fn get_domains_passive(&self) -> &Vec<Domain> {
        &self.domains
    }

    /// Returns a mutable reference to the CPU cores vector.
    fn get_cores(&mut self) -> &mut Vec<CPUCore> {
        &mut self.cpu_cores
    }

    /// Returns a immutable reference to the CPU cores vector.
    fn get_cores_passive(&self) -> &Vec<CPUCore> {
        &self.cpu_cores
    }

    fn get_stat_buffer(&mut self) -> &mut Vec<CPUStat> {
        &mut self.stat_buffer
    }

    fn get_stat_buffer_passive(&self) -> &Vec<CPUStat> {
        &self.stat_buffer
    }

    fn get_debug_type(&self) -> String {
        String::from("Debug")
    }
}