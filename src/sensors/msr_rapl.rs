use crate::sensors::utils::current_system_time_since_epoch;
use crate::sensors::{CPUSocket, Domain, Record, RecordReader, Sensor, Topology};
use core::ffi::c_void;
use std::error::Error;
use std::mem::{size_of, size_of_val};
use sysinfo::{ProcessorExt, System, SystemExt};
use windows::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ACCESS_FLAGS, FILE_FLAG_OVERLAPPED, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
    FILE_READ_DATA, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Ioctl::{FILE_DEVICE_UNKNOWN, METHOD_OUT_DIRECT};
use windows::Win32::System::IO::{DeviceIoControl, OVERLAPPED};

const AGENT_POWER_UNIT_CODE: u16 = 0xBEB;
const AGENT_POWER_LIMIT_CODE: u16 = 0xBEC;
const AGENT_ENERGY_STATUS_CODE: u16 = 0xBED;

unsafe fn send_request(device: HANDLE, request_code: u16,
    request: *const u8, request_length: usize,
    reply: *mut u8, reply_length: usize) -> bool {
    let i: usize;
    let mut len: u32 = 0;
    let len_ptr: *mut u32 = &mut len; 

    let slice = std::slice::from_raw_parts_mut(reply, reply_length);
    slice.fill(0); //memset(reply, 0, replyLength);

    if DeviceIoControl(
        device, // envoi 8 octet et je recoi 8 octet
        crate::sensors::msr_rapl::CTL_CODE(
            FILE_DEVICE_UNKNOWN, request_code as _,
            METHOD_OUT_DIRECT, FILE_READ_DATA.0
            // nouvelle version : METHOD_OUD_DIRECT devien METHOD_BUFFERED
        ),
        request as _, request_length as u32,
        reply as _, reply_length as u32,
        len_ptr, std::ptr::null_mut()
    ).as_bool() {
        if len != reply_length as u32 {
            error!("Got invalid answer length, Expected {}, got {}", reply_length, len);
        }
        warn!("Device answered !");
        true 
    } else {
        false
    }
}
    
unsafe fn CTL_CODE(device_type: u32, request_code: u32, method: u32, access: u32) -> u32 {
    let res = ((device_type) << 16) | ((access) << 14) | ((request_code) << 2) | (method);
    println!("device_type: {}, access: {:?}, request_code: {}, method: {}", device_type, access, request_code, method);
    println!("res: {}", res);
    res
}

pub fn extract_rapl_current_power(data: u64) -> String {
    let mut energy_consumed: u64 = 0;
    warn!("{}", data);
    energy_consumed = (data & 0xFFFFFFFF)*100;
    warn!("Current power usage: {} microJ\n", energy_consumed);
    //println!("Current power usage: {} Watts\n", ((energy_consumed - energy_consumed_previous) / 1) / 1000000);
    format!("{}", energy_consumed)
}

pub unsafe fn get_handle(driver_name: &str) -> Result<HANDLE, String> {
    let device: HANDLE;
    device = CreateFileW(
        driver_name,
        FILE_GENERIC_READ | FILE_GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        std::ptr::null_mut(),
        OPEN_EXISTING,
        FILE_FLAG_OVERLAPPED,
        None);
    if device == INVALID_HANDLE_VALUE {
        error!("Failed to open device : {:?}", device);
        return Err(String::from("Couldn't get handle got INVALIDE_HANDLE_VALUE"))
    }
    info!("Device opened : {:?}", device);
    Ok(device)
}

pub struct MsrRAPLSensor {
    driver_name: String,
}

impl MsrRAPLSensor {
    pub fn new() -> MsrRAPLSensor {
       
        let driver_name = "\\\\.\\RAPLDriver";
        
        MsrRAPLSensor {
            driver_name: String::from(driver_name),
        }
    }    
}

impl RecordReader for Topology {
    fn read_record(&self) -> Result<Record, Box<dyn Error>> {
        let randval: i32 = rand::random();
        Ok(Record {
            timestamp: current_system_time_since_epoch(),
            unit: super::units::Unit::MicroJoule,
            value: format!("{}", randval),
        })
    }
}
impl RecordReader for CPUSocket {
    fn read_record(&self) -> Result<Record, Box<dyn Error>> {
        unsafe {
            if let Ok(device) = crate::sensors::msr_rapl::get_handle(&self.source) {
                let mut msr_result = [0u8; size_of::<u64>()];
                        
                if send_request(device, AGENT_ENERGY_STATUS_CODE,
                    // nouvelle version à integrer : request_code est ignoré et request doit contenir
                    // request_code sous forme d'un char *
                    std::ptr::null(), 0,
                    msr_result.as_mut_ptr(), size_of::<u64>()
                ) {
                    warn!("msr_result: {:?}", msr_result);
                    let mut arr = [0u8; 8];
                    arr.copy_from_slice(&msr_result);
                    Ok(Record {
                        timestamp: current_system_time_since_epoch(),
                        unit: super::units::Unit::MicroJoule,
                        value: crate::sensors::msr_rapl::extract_rapl_current_power(u64::from_ne_bytes(arr)),
                    })
                } else {
                    error!("Failed to get data from send_request.");
                    Ok(Record {
                        timestamp: current_system_time_since_epoch(),
                        unit: super::units::Unit::MicroJoule,
                        value: String::from("0"),
                    })
                }
            } else {
                error!("Couldn't get handle.");
                Ok(Record {
                    timestamp: current_system_time_since_epoch(),
                    unit: super::units::Unit::MicroJoule,
                    value: String::from("0"),
                })
            }
        }
    }
}
impl RecordReader for Domain {
    fn read_record(&self) -> Result<Record, Box<dyn Error>> {
        Ok(Record {
            timestamp: current_system_time_since_epoch(),
            unit: super::units::Unit::MicroJoule,
            value: String::from("10"),
        })
    }
}

impl Sensor for MsrRAPLSensor {
    fn generate_topology(&self) -> Result<Topology, Box<dyn Error>> {
        let mut topology = Topology::new(self.driver_name.clone());
        let mut sys = System::new_all();
        sys.refresh_all();
        let mut i = 0;
        //TODO fix that to actually count the number of sockets
        topology.safe_add_socket(
            i,
            vec![],
            vec![],
            String::from(""),
            4,
            self.driver_name.clone(),
        );

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
