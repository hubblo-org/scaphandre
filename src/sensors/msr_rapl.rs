use crate::sensors::{Sensor, Topology, CPUSocket, Domain, RecordReader, Record};
use crate::sensors::utils::current_system_time_since_epoch;
use core::ffi::c_void;
use std::error::Error;
use std::mem::{size_of, size_of_val};
use windows::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ACCESS_FLAGS, FILE_FLAG_OVERLAPPED, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
    FILE_READ_DATA, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Ioctl::{FILE_DEVICE_UNKNOWN, METHOD_OUT_DIRECT};
use windows::Win32::System::IO::{DeviceIoControl, OVERLAPPED};
use sysinfo::{System, SystemExt, ProcessorExt};

const AGENT_POWER_UNIT_CODE: u16 = 0xBEB;
const AGENT_POWER_LIMIT_CODE: u16 = 0xBEC;
const AGENT_ENERGY_STATUS_CODE: u16 = 0xBED;

pub struct MsrRAPLSensor {
    driver_name: String,
}

impl MsrRAPLSensor {
    pub fn new() -> MsrRAPLSensor {
        MsrRAPLSensor {
            driver_name: String::from("\\\\.\\RAPLDriver"),
        }
    }
}

impl RecordReader for Topology {
    fn read_record(&self) -> Result<Record, Box<dyn Error>> {
        let randval: i32 = rand::random();
        Ok(
            Record {
                timestamp: current_system_time_since_epoch(),
                unit: super::units::Unit::MicroJoule,
                value: format!("{}", randval)
            }
        )
    }
}
impl RecordReader for CPUSocket {
    fn read_record(&self) -> Result<Record, Box<dyn Error>> {
        let randval: i32 = 0;
        Ok(
            Record {
                timestamp: current_system_time_since_epoch(),
                unit: super::units::Unit::MicroJoule,
                value: format!("{}", randval)
            }
        )
    }
}
impl RecordReader for Domain {
    fn read_record(&self) -> Result<Record, Box<dyn Error>> {
        Ok(
            Record {
                timestamp: current_system_time_since_epoch(),
                unit: super::units::Unit::MicroJoule,
                value: String::from("10")
            }
        )
    }
}


impl Sensor for MsrRAPLSensor {
    fn generate_topology(&self) -> Result<Topology, Box<dyn Error>> {
        let mut topology = Topology::new(self.driver_name.clone());
        let mut sys = System::new_all();
        sys.refresh_all();
        let mut i = 0;
        for socket in sys.processors(){
            topology.safe_add_socket(
                i,
                vec![],
                vec![],
                String::from(""),
                4,
                self.driver_name.clone()
            );

            i += 1;
        }
        Ok(topology)
    }

    fn get_topology(&mut self) -> Box<Option<Topology>> {
        let topology = self.generate_topology().ok();
        if topology.is_none() {
            panic!("Couldn't generate the topology !");
        }
        Box::new(topology)
    }
}
