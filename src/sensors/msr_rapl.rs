use crate::sensors::utils::current_system_time_since_epoch;
use crate::sensors::{CPUSocket, Domain, Record, RecordReader, Sensor, Topology};
use core::ffi::c_void;
use std::error::Error;
use std::mem::{transmute, size_of, size_of_val};
use sysinfo::{ProcessorExt, System, SystemExt};
use windows::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ACCESS_FLAGS, FILE_FLAG_OVERLAPPED, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
    FILE_READ_DATA, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING, FILE_WRITE_DATA
};
use windows::Win32::System::Ioctl::{FILE_DEVICE_UNKNOWN, METHOD_OUT_DIRECT, METHOD_BUFFERED};
use windows::Win32::System::IO::{DeviceIoControl, OVERLAPPED};

const AGENT_POWER_UNIT_CODE: u16 = 0x606;
const AGENT_POWER_LIMIT_CODE: u16 = 0x610;
const AGENT_ENERGY_STATUS_CODE: u16 = 0x611;

    
unsafe fn ctl_code(device_type: u32, request_code: u32, method: u32, access: u32) -> u32 {
    let res = ((device_type) << 16) | ((access) << 14) | ((request_code) << 2) | (method);
    println!("device_type: {}, access: {:?}, request_code: {}, method: {}", device_type, access, request_code, method);
    println!("control colde : {}", res);
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

pub fn extract_rapl_power_units(data: u64){
    // Intel documentation says high level bits are reserved, so ignore them
    let new_data: u64;
	//uint16_t time;
    let time: u64;
	//uint16_t power;
    let power: u64;
	//uint32_t energy;
    let energy: u64;
	//double time_units;
    let time_units: i32; 
	//double power_units;
    let power_units: i32;
	//double energy_units;
    let energy_units: i32;

	new_data = data & 0xFFFFFFFF;

	//// Power units are located from bits 0 to 3, extract them
	power = new_data & 0x0F;

	//// Energy state units are located from bits 8 to 12, extract them
	energy = (new_data >> 8) & 0x1F;

	//// Time units are located from bits 16 to 19, extract them
	time = (new_data >> 16) & 0x0F;

	//// Intel documentation says: 1 / 2^power
	//power_units = 1.0 / pow(2, static_cast<double>(power));
    let divider = i32::pow(power as i32, 2);
    println!("divider: {}", divider);
	power_units = 1 / divider;

	//// Intel documentation says: 1 / 2^energy
	//energy_units = 1.0 / pow(2, static_cast<double>(energy));
    let divider = i32::pow(energy as i32, 2);
    println!("divider: {}", divider);
    energy_units = 1 / divider;

	//// Intel documentation says: 1 / 2^energy
	//time_units = 1.0 / pow(2, static_cast<double>(time));
    let divider = i32::pow(time as i32, 2);
    println!("divider: {}", divider);
    time_units = 1 / divider;

	println!("CPU energy unit is: {} microJ\n", energy_units * 1000000);
	println!("CPU power unit is: {} Watt(s)\n", power_units);
	println!("CPU time unit is: {} second(s)\n", time_units);
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

pub unsafe fn get_rapl_energy_unit(device: HANDLE) -> Result<Record, String> {
    warn!("ENERGY UNIT ########################### START");
    let mut msr_result = [0u8; size_of::<u64>()];
            
    match send_request(device, AGENT_POWER_UNIT_CODE,
        // nouvelle version à integrer : request_code est ignoré et request doit contenir
        // request_code sous forme d'un char *
        std::ptr::null(), 0,
        msr_result.as_mut_ptr(), size_of::<u64>()
    ) {
        Ok(res) => {
            warn!("msr_result: {:?}", msr_result);
            let mut arr = [0u8; 8];
            arr.copy_from_slice(&msr_result);
            warn!("arr: {:?}", arr);
            warn!("from ne bytes arr: {:?}", u64::from_ne_bytes(arr));
            crate::sensors::msr_rapl::extract_rapl_power_units(u64::from_ne_bytes(arr));
            //crate::sensors::msr_rapl::close_handle(device);
            warn!("ENERGY UNIT ########################### END");
            Ok(Record {
                timestamp: current_system_time_since_epoch(),
                unit: super::units::Unit::MicroJoule,
                //value: format!("{}", u64::from_ne_bytes(arr)*100),
                value: crate::sensors::msr_rapl::extract_rapl_current_power(u64::from_le_bytes(arr)),
            })
        },
        Err(err) => {
            error!("Failed to get data from send_request.");
            //crate::sensors::msr_rapl::close_handle(device);
            warn!("ENERGY UNIT ########################### END");
            Err(String::from(""))
        }
    }
}

pub unsafe fn close_handle(handle: HANDLE) {
    let res = CloseHandle(handle);
    if res.as_bool() {
        debug!("Device closed.")
    } else {
        error!("Failed to close device.");
    }
}

pub fn convert(value: [u16; 4]) -> u64 {
    let [a, b] = value[0].to_le_bytes();
    let [c, d] = value[1].to_le_bytes();
    let [e, f] = value[2].to_le_bytes();
    let [g, h] = value[3].to_le_bytes();
    u64::from_le_bytes([a, b, c, d, e, f, g, h])
}

pub struct MsrRAPLSensor {
    driver_name: String,
}

impl MsrRAPLSensor {
    pub fn new() -> MsrRAPLSensor {
       
        let driver_name = "\\\\.\\ScaphandreDriver";
        
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

unsafe fn send_request(device: HANDLE, request_code: u16,
    request: *const u64, request_length: usize,
    reply: *mut u8, reply_length: usize) -> Result<String, String> {
    let mut len: u32 = 0;
    let len_ptr: *mut u32 = &mut len; 

    if DeviceIoControl(
        device, // envoi 8 octet et je recoi 8 octet
        crate::sensors::msr_rapl::ctl_code(
            FILE_DEVICE_UNKNOWN, 0x801 as _,
            METHOD_BUFFERED, FILE_READ_DATA.0 | FILE_WRITE_DATA.0
            // nouvelle version : METHOD_OUD_DIRECT devien METHOD_BUFFERED
        ),
        request as _, request_length as u32,
        reply as _, reply_length as u32,
        len_ptr, std::ptr::null_mut()
    ).as_bool() {
        if len != reply_length as u32 {
            error!("Got invalid answer length, Expected {}, got {}", reply_length, len);
        }
        info!("Device answered");
        Ok(String::from("Device answered !"))
    } else {
        error!("DeviceIoControl failed");
        Err(String::from("DeviceIoControl failed"))
    }
}
impl RecordReader for CPUSocket {
    fn read_record(&self) -> Result<Record, Box<dyn Error>> {
        unsafe {
            if let Ok(device) = crate::sensors::msr_rapl::get_handle(&self.source) {
                let mut msr_result = [0u8; size_of::<u64>()];

                warn!("AGENT_ENERGY_STATUS_CODE: {:b}", AGENT_ENERGY_STATUS_CODE);

                //1100 0010  0010 0000  0000 0000  0000 0000  0000 0000  0000 0000  0000 0000 0000 0000
                //let src = (AGENT_ENERGY_STATUS_CODE as u64) << 47;
                let src = AGENT_ENERGY_STATUS_CODE as u64;
                warn!("src: {:x}",src);
                warn!("src: {:b}",src);

                let ptr = &src as *const u64;
                warn!("*ptr: {}", *ptr);
                warn!("&request: {:?} ptr (as *const u8): {:?}", &src, ptr);

                if let Ok(res) = send_request(device, AGENT_ENERGY_STATUS_CODE,
                    // nouvelle version à integrer : request_code est ignoré et request doit contenir
                    // request_code sous forme d'un char *
                    ptr, 8,
                    msr_result.as_mut_ptr(), size_of::<u64>()
                ) {
                    warn!("msr_result: {:?}", msr_result);
                    let mut arr = [0u8; 8];
                    arr.copy_from_slice(&msr_result);
                    warn!("arr: {:?}", arr);
                    warn!("from ne bytes arr: {:?}", u64::from_ne_bytes(arr));
                    crate::sensors::msr_rapl::close_handle(device);
                    Ok(Record {
                        timestamp: current_system_time_since_epoch(),
                        unit: super::units::Unit::MicroJoule,
                        //value: format!("{}", u64::from_ne_bytes(arr)*100),
                        value: crate::sensors::msr_rapl::extract_rapl_current_power(u64::from_le_bytes(arr)),
                    })
                } else {
                    error!("Failed to get data from send_request.");
                    crate::sensors::msr_rapl::close_handle(device);
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
        let i = 0;
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
