use crate::sensors::utils::current_system_time_since_epoch;
use crate::sensors::{CPUSocket, Domain, Record, RecordReader, Sensor, Topology};
use std::collections::HashMap;
use std::error::Error;
use std::mem::size_of;
use sysinfo::{System, SystemExt};
use windows::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_OVERLAPPED, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_READ_DATA,
    FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_WRITE_DATA, OPEN_EXISTING,
};
use windows::Win32::System::Ioctl::{FILE_DEVICE_UNKNOWN, METHOD_BUFFERED};
use windows::Win32::System::IO::DeviceIoControl;

const MSR_RAPL_POWER_UNIT: u16 = 0x606; //
                                        //const MSR_PKG_POWER_LIMIT: u16 = 0x610; // PKG RAPL Power Limit Control (R/W) See Section 14.7.3, Package RAPL Domain.
const MSR_PKG_ENERGY_STATUS: u16 = 0x611;
//const MSR_PKG_POWER_INFO: u16 = 0x614;
//const MSR_DRAM_ENERGY_STATUS: u16 = 0x619;
//const MSR_PP0_ENERGY_STATUS: u16 = 0x639; //PP0 Energy Status (R/O) See Section 14.7.4, PP0/PP1 RAPL Domains.
//const MSR_PP0_PERF_STATUS: u16 = 0x63b; // PP0 Performance Throttling Status (R/O) See Section 14.7.4, PP0/PP1 RAPL Domains.
//const MSR_PP0_POLICY: u16 = 0x63a; //PP0 Balance Policy (R/W) See Section 14.7.4, PP0/PP1 RAPL Domains.
//const MSR_PP0_POWER_LIMIT: u16 = 0x638; // PP0 RAPL Power Limit Control (R/W) See Section 14.7.4, PP0/PP1 RAPL Domains.
//const MSR_PP1_ENERGY_STATUS: u16 = 0x641; // PP1 Energy Status (R/O) See Section 14.7.4, PP0/PP1 RAPL Domains.
//const MSR_PP1_POLICY: u16 = 0x642; // PP1 Balance Policy (R/W) See Section 14.7.4, PP0/PP1 RAPL Domains.
//const MSR_PP1_POWER_LIMIT: u16 = 0x640; // PP1 RAPL Power Limit Control (R/W) See Section 14.7.4, PP0/PP1 RAPL Domains.

unsafe fn ctl_code(device_type: u32, request_code: u32, method: u32, access: u32) -> u32 {
    ((device_type) << 16) | ((access) << 14) | ((request_code) << 2) | (method)
}

/// # Safety
///
/// Unsafe code due to direct calls to Windows API.
pub unsafe fn get_handle(driver_name: &str) -> Result<HANDLE, String> {
    let device: HANDLE = CreateFileW(
        driver_name,
        FILE_GENERIC_READ | FILE_GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        std::ptr::null_mut(),
        OPEN_EXISTING,
        FILE_FLAG_OVERLAPPED,
        None,
    );
    if device == INVALID_HANDLE_VALUE {
        error!("Failed to open device : {:?}", device);
        return Err(format!("Got Last Error : {:?}", GetLastError()));
    }
    info!("Device opened : {:?}", device);
    Ok(device)
}

/// # Safety
///
/// Unsafe code due to direct calls to Windows API.
pub unsafe fn close_handle(handle: HANDLE) {
    let res = CloseHandle(handle);
    if res.as_bool() {
        debug!("Device closed.")
    } else {
        error!("Failed to close device.");
    }
}

pub struct MsrRAPLSensor {
    driver_name: String,
    power_unit: f64,
    energy_unit: f64,
    time_unit: f64,
}

impl Default for MsrRAPLSensor {
    fn default() -> Self {
        Self::new()
    }
}

impl MsrRAPLSensor {
    pub fn new() -> MsrRAPLSensor {
        let driver_name = "\\\\.\\ScaphandreDriver";

        let mut power_unit: f64 = 1.0;
        let mut energy_unit: f64 = 1.0;
        let mut time_unit: f64 = 1.0;

        unsafe {
            if let Ok(device) = get_handle(driver_name) {
                let mut msr_result: u64 = 0;
                let ptr_result = &mut msr_result as *mut u64;
                let src = MSR_RAPL_POWER_UNIT as u64;
                let ptr = &src as *const u64;
                if let Ok(res) = send_request(
                    device,
                    MSR_RAPL_POWER_UNIT,
                    ptr,
                    8,
                    ptr_result,
                    size_of::<u64>(),
                ) {
                    debug!("{}", res);
                    power_unit = MsrRAPLSensor::extract_rapl_power_unit(msr_result);
                    energy_unit = MsrRAPLSensor::extract_rapl_energy_unit(msr_result);
                    time_unit = MsrRAPLSensor::extract_rapl_time_unit(msr_result);
                } else {
                    warn!("Couldn't get RAPL units !");
                }

                close_handle(device);
            }
        }

        MsrRAPLSensor {
            driver_name: String::from(driver_name),
            energy_unit,
            power_unit,
            time_unit,
        }
    }

    pub fn extract_rapl_power_unit(data: u64) -> f64 {
        // Intel documentation says high level bits are reserved, so ignore them
        let new_data: u32 = (data & 0xFFFFFFFF) as u32;
        //// Power units are located from bits 0 to 3, extract them
        let power: u32 = new_data & 0x0F;

        //// Intel documentation says: 1 / 2^power
        let divider = i64::pow(2, power);

        1.0 / divider as f64
    }
    pub fn extract_rapl_energy_unit(data: u64) -> f64 {
        // Intel documentation says high level bits are reserved, so ignore them
        let new_data: u32 = (data & 0xFFFFFFFF) as u32;
        //// Energy state units are located from bits 8 to 12, extract them
        let energy: u32 = (new_data >> 8) & 0x1F;

        //// Intel documentation says: 1 / 2^power
        let divider = i64::pow(2, energy);

        1.0 / divider as f64
    }
    pub fn extract_rapl_time_unit(data: u64) -> f64 {
        // Intel documentation says high level bits are reserved, so ignore them
        let new_data: u32 = (data & 0xFFFFFFFF) as u32;
        //// Time units are located from bits 16 to 19, extract them
        let time: u32 = (new_data >> 16) & 0x0F;

        //// Intel documentation says: 1 / 2^power
        let divider = i64::pow(2, time);

        1.0 / divider as f64
    }

    pub fn extract_rapl_current_power(data: u64, energy_unit: f64) -> String {
        let energy_consumed: f64 = ((data & 0xFFFFFFFF) as f64) * energy_unit * 1000000.0;
        format!("{}", energy_consumed as u64)
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

unsafe fn send_request(
    device: HANDLE,
    request_code: u16,
    request: *const u64,
    request_length: usize,
    reply: *mut u64,
    reply_length: usize,
) -> Result<String, String> {
    let mut len: u32 = 0;
    let len_ptr: *mut u32 = &mut len;

    if DeviceIoControl(
        device, // envoi 8 octet et je recoi 8 octet
        crate::sensors::msr_rapl::ctl_code(
            FILE_DEVICE_UNKNOWN,
            request_code as _,
            METHOD_BUFFERED,
            FILE_READ_DATA.0 | FILE_WRITE_DATA.0, // nouvelle version : METHOD_OUD_DIRECT devien METHOD_BUFFERED
        ),
        request as _,
        request_length as u32,
        reply as _,
        reply_length as u32,
        len_ptr,
        std::ptr::null_mut(),
    )
    .as_bool()
    {
        if len != reply_length as u32 {
            error!(
                "Got invalid answer length, Expected {}, got {}",
                reply_length, len
            );
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
            let driver_name = self.sensor_data.get("DRIVER_NAME").unwrap();
            if let Ok(device) = get_handle(driver_name) {
                let mut msr_result: u64 = 0;
                let ptr_result = &mut msr_result as *mut u64;
                let mut src = MSR_RAPL_POWER_UNIT as u64;
                let ptr = &src as *const u64;

                src = MSR_PKG_ENERGY_STATUS as u64;
                trace!("src: {:x}", src);
                trace!("src: {:b}", src);

                trace!("*ptr: {}", *ptr);
                trace!("&request: {:?} ptr (as *const u8): {:?}", &src, ptr);

                if let Ok(res) = send_request(
                    device,
                    MSR_PKG_ENERGY_STATUS,
                    // nouvelle version à integrer : request_code est ignoré et request doit contenir
                    // request_code sous forme d'un char *
                    ptr,
                    8,
                    ptr_result,
                    size_of::<u64>(),
                ) {
                    debug!("{}", res);

                    close_handle(device);

                    let energy_unit = self
                        .sensor_data
                        .get("ENERGY_UNIT")
                        .unwrap()
                        .parse::<f64>()
                        .unwrap();

                    Ok(Record {
                        timestamp: current_system_time_since_epoch(),
                        unit: super::units::Unit::MicroJoule,
                        value: MsrRAPLSensor::extract_rapl_current_power(msr_result, energy_unit),
                    })
                } else {
                    error!("Failed to get data from send_request.");
                    close_handle(device);
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
        let mut sensor_data = HashMap::new();
        sensor_data.insert(String::from("DRIVER_NAME"), self.driver_name.clone());
        sensor_data.insert(String::from("ENERGY_UNIT"), self.energy_unit.to_string());
        sensor_data.insert(String::from("POWER_UNIT"), self.power_unit.to_string());
        sensor_data.insert(String::from("TIME_UNIT"), self.time_unit.to_string());

        let mut topology = Topology::new(sensor_data.clone());
        let mut sys = System::new_all();
        sys.refresh_all();
        let i = 0;
        //TODO fix that to actually count the number of sockets
        topology.safe_add_socket(i, vec![], vec![], String::from(""), 4, sensor_data.clone());

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
