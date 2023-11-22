use crate::sensors::utils::current_system_time_since_epoch;
use crate::sensors::{CPUSocket, Domain, Record, RecordReader, Sensor, Topology, CPUCore};
use std::collections::HashMap;
use std::error::Error;
use std::mem::size_of;
use sysinfo::{System, SystemExt, CpuExt, Cpu};
use raw_cpuid::{CpuId, TopologyType};
use windows::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_OVERLAPPED, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_READ_DATA,
    FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_WRITE_DATA, OPEN_EXISTING,
};
use windows::Win32::System::Ioctl::{FILE_DEVICE_UNKNOWN, METHOD_BUFFERED};
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::System::Threading::{
    GetThreadGroupAffinity, GetProcessGroupAffinity, GetCurrentProcess, GetProcessInformation,
    GetCurrentThread, GetActiveProcessorGroupCount, SetThreadGroupAffinity
};
use windows::Win32::System::SystemInformation::GROUP_AFFINITY;

use core_affinity::{self, CoreId};

use x86::cpuid;
// Intel RAPL MSRs
use x86::msr::{
    MSR_RAPL_POWER_UNIT,
    MSR_PKG_POWER_LIMIT,
    MSR_PKG_POWER_INFO,
    MSR_PKG_ENERGY_STATUS,
    MSR_DRAM_ENERGY_STATUS,
    MSR_DRAM_PERF_STATUS,
    MSR_PP0_ENERGY_STATUS,
    MSR_PP0_PERF_STATUS,
    MSR_PP1_ENERGY_STATUS,
};
const MSR_PLATFORM_ENERGY_STATUS: u32 = 0x0000064d;
const MSR_PLATFORM_POWER_LIMIT: u32 = 0x0000065c ;

// AMD RAPL MSRs
const MSR_AMD_RAPL_POWER_UNIT: u32 = 0xc0010299;
const MSR_AMD_CORE_ENERGY_STATUS: u32 = 0xc001029a;
const MSR_AMD_PKG_ENERGY_STATUS: u32 = 0xc001029b;


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
        let mut res: u64 = 0;
        warn!("Topology: I have {} sockets", self.sockets.len());
        for s in &self.sockets {
            match s.read_record() {
                Ok(rec) => {
                    warn!("rec: {:?}", rec);
                    res = res + rec.value.parse::<u64>()?;
                },
                Err(e) => {
                    error!("Failed to get socket record : {:?}", e);
                }
            }
        }
        Ok(Record {
            timestamp: current_system_time_since_epoch(),
            unit: super::units::Unit::MicroJoule,
            value: res.to_string(),
        })
    }
}

unsafe fn send_request(
    device: HANDLE,
    request_code: u32,
    request: *const u64,
    request_length: usize,
    reply: *mut u64,
    reply_length: usize,
) -> Result<String, String> {
    let mut len: u32 = 0;
    let len_ptr: *mut u32 = &mut len;

    if DeviceIoControl(
        device, // send 8 bytes, receive 8 bytes
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
            let current_thread = GetCurrentThread();
            let processorgroup_id = self.sensor_data.get("PROCESSORGROUP_ID").unwrap().parse::<u16>().unwrap();
            let mut thread_group_affinity: GROUP_AFFINITY = GROUP_AFFINITY { Mask: 255, Group: processorgroup_id, Reserved: [0,0,0] };
            let thread_affinity = GetThreadGroupAffinity(current_thread, &mut thread_group_affinity);
            if thread_affinity.as_bool() {
                warn!("got thead_affinity : {:?}", thread_group_affinity);
                let core_id = self.cpu_cores.last().unwrap().id; //(self.cpu_cores.last().unwrap().id + self.id * self.cpu_cores.len() as u16) as usize
                let newaffinity = GROUP_AFFINITY { Mask: (self.cpu_cores.len() + self.id as usize * self.cpu_cores.len() - 1) as usize, Group: processorgroup_id, Reserved: [0, 0, 0]};
                let res = SetThreadGroupAffinity(current_thread, &newaffinity, &mut thread_group_affinity);                
                if res.as_bool() {
                    warn!("Asking get_msr_value, from socket, with core_id={}", core_id);
                    match get_msr_value(core_id as usize, MSR_PKG_ENERGY_STATUS as u64, &self.sensor_data) {
                        Ok(rec) => {
                            return Ok(Record { timestamp: current_system_time_since_epoch(), value: rec.value, unit: super::units::Unit::MicroJoule })
                        },
                        Err(e) => {
                            error!("Could'nt get MSR value for {}: {}", MSR_PKG_ENERGY_STATUS, e);
                            return Ok(Record { 
                                timestamp: current_system_time_since_epoch(),
                                value: String::from("0"),
                                unit: super::units::Unit::MicroJoule
                            })
                        }
                    }
                } else {
                    panic!("Couldn't set Thread affinity !");
                }
            } else {
                panic!("Coudld'nt get Thread affinity !");
            }
        }
    }
}
impl RecordReader for Domain {
    fn read_record(&self) -> Result<Record, Box<dyn Error>> {
        if let Some(core_id) = self.sensor_data.get("CORE_ID") {
            let usize_coreid = core_id.parse::<usize>().unwrap();
            warn!("Reading Domain {} on Core {}", self.name, usize_coreid);
            if let Some(msr_addr) = self.sensor_data.get("MSR_ADDR") {
                unsafe {
                    warn!("Asking, from Domain, get_msr_value with core_id={}", usize_coreid);
                    match get_msr_value(usize_coreid, msr_addr.parse::<u64>().unwrap(), &self.sensor_data) {
                        Ok(rec) => {
                            return Ok(Record {
                                timestamp: current_system_time_since_epoch(),
                                unit: super::units::Unit::MicroJoule,
                                value: rec.value,
                            })
                        },
                        Err(e) => {
                            error!("Could'nt get MSR value for {}: {}", msr_addr, e);
                            Ok(Record { 
                                timestamp: current_system_time_since_epoch(),
                                value: String::from("0"),
                                unit: super::units::Unit::MicroJoule
                            })
                        }
                    }
                }
            } else {
                panic!("Couldn't get msr_addr to target for domain {}", self.name);
            }
        } else {
            panic!("Couldn't get core_id to target for domain {}", self.name);
        }
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

        unsafe {
            let current_thread = GetCurrentThread();

            let group_count = GetActiveProcessorGroupCount();
            warn!("GROUP COUNT : {}", group_count);

            for group_id in 0..group_count {
                //TODO fix that to actually count the number of sockets
                let logical_cpus = sys.cpus() ;
                let mut nb_cpu_sockets: u16 = 0;
                let cpuid = CpuId::new();
                let mut logical_cpus_from_cpuid = 1;
                match cpuid.get_extended_topology_info() {
                    Some(info) => {
                        for t in info {
                            if t.level_type() == TopologyType::Core {
                                logical_cpus_from_cpuid = t.processors();
                            }
                        }
                    },
                    None => {
                        panic!("Could'nt get cpuid data.");
                    }
                }
                if logical_cpus_from_cpuid <= 1 {
                    panic!("CpuID data is likely to be wrong.");
                }
                let mut i: u16 = 0;
                let mut no_more_sockets = false;
                warn!("Entering ProcessorGroup {}", group_id);
                let newaffinity = GROUP_AFFINITY { Mask: 255, Group: group_id, Reserved: [0, 0, 0]};
                let mut thread_group_affinity: GROUP_AFFINITY = GROUP_AFFINITY { Mask: 255, Group: 0, Reserved: [0,0,0] };
                let thread_affinity = GetThreadGroupAffinity(current_thread, &mut thread_group_affinity);
                warn!("Thread group affinity result : {:?}", thread_affinity);
                if thread_affinity.as_bool() {
                    warn!("got thead_affinity : {:?}", thread_group_affinity);
                    let res = SetThreadGroupAffinity(current_thread, &newaffinity, &mut thread_group_affinity);                
                    if res.as_bool() {
                        warn!("Have set thread affinity: {:?}", newaffinity);
                        match core_affinity::get_core_ids() {
                            Some(core_ids) => {
                                warn!("CPU SETUP - Cores from core_affinity, len={} : {:?}", core_ids.len(), core_ids);
                                warn!("CPU SETUP - Logical CPUs from sysinfo: {}", logical_cpus.len());
                                while !no_more_sockets {
                                    let start = i * logical_cpus_from_cpuid;
                                    let stop = (i+1)*logical_cpus_from_cpuid;
                                    warn!("Looping over {} .. {}", start, stop);
                                    sensor_data.insert(String::from("PROCESSORGROUP_ID"), group_id.to_string());
                                    let mut current_socket = CPUSocket::new(i, vec![], vec![], String::from(""),1,  sensor_data.clone());
                                    for c in start..stop {//core_ids {
                                        if core_affinity::set_for_current(CoreId { id: c.into() }) {
                                            match cpuid.get_vendor_info() {
                                                Some(info) => {
                                                    warn!("Got CPU {:?}", info);
                                                },
                                                None => {
                                                    warn!("Couldn't get cpuinfo");
                                                }
                                            }
                                            warn!("Set core_affinity to {}", c);
                                            match cpuid.get_extended_topology_info() {
                                                Some(info) => {
                                                    warn!("Got CPU topo info {:?}", info);
                                                    for t in info {
                                                        if t.level_type() == TopologyType::Core {
                                                            //nb_cpu_sockets = logical_cpus.len() as u16 / t.processors();
                                                            //logical_cpus_from_cpuid = t.processors()
                                                            let x2apic_id = t.x2apic_id();
                                                            let socket_id = (x2apic_id & 240) >> 4; // upper bits of x2apic_id are socket_id, mask them, then bit shift to get socket_id
                                                            current_socket.set_id(socket_id as u16);
                                                            let core_id = x2apic_id & 15; // 4 last bits of x2apic_id are the core_id (per-socket)
                                                            warn!("Found socketid={} and coreid={}", socket_id, core_id);
                                                            let mut attributes = HashMap::<String, String>::new();
                                                            let ref_core = logical_cpus.first().unwrap();
                                                            attributes.insert(String::from("frequency"), ref_core.frequency().to_string());
                                                            attributes.insert(String::from("name"), ref_core.name().to_string());
                                                            attributes.insert(String::from("vendor_id"), ref_core.vendor_id().to_string());
                                                            attributes.insert(String::from("brand"), ref_core.brand().to_string());
                                                            warn!("Adding core id {} to socket_id {}", ((i * (logical_cpus_from_cpuid - 1)) + core_id as u16), current_socket.id);
                                                            current_socket.add_cpu_core(CPUCore::new((i * (logical_cpus_from_cpuid - 1)) + core_id as u16, attributes));
                                                            warn!("Reviewing sockets : {:?}", topology.get_sockets_passive());
                                                        }
                                                    }
                                                },
                                                None => {
                                                    warn!("Couldn't get cpu topo info");
                                                }
                                            }
                                        } else {
                                            no_more_sockets = true;
                                            warn!("There's likely to be no more socket to explore.");
                                            break;
                                        }
                                    }    
                                    if !no_more_sockets {
                                        warn!("inserting socket {:?}", current_socket);
                                        topology.safe_insert_socket(current_socket);
                                        i = i + 1;
                                    }
                                }
                                nb_cpu_sockets = i;
                            },
                            None => {
                                panic!("Could'nt get core ids from core_affinity.");
                            }
                        }
                        if let Some(info) = CpuId::new().get_extended_topology_info() {
                            for c in info {
                                if c.level_type() == TopologyType::Core {
                                    warn!("CPUID : {:?}", c);
                                }
                            }
                        }
                    } else {
                        error!("Could'nt set thread affinity !");
                        let last_error = GetLastError();
                        panic!("Error was : {:?}", last_error);
                    }
                } else {
                    warn!("Getting thread group affinity failed !");
                    let last_error = GetLastError();
                    panic!("Error was: {:?}", last_error); // win32 error 122 is insufficient buffer
                }
            }
            //let process_information = GetProcessInformation(current_process, , , );
        }
        //nb_cpu_sockets = logical_cpus.len() as u16 / logical_cpus_from_cpuid;
        //let mut core_id_counter = logical_cpus.len();

        //match cpuid.get_advanced_power_mgmt_info() {
        //    Some(info) => {
        //        warn!("Got CPU power mgmt info {:?}", info);
        //    },
        //    None => {
        //        warn!("Couldn't get cpu power info");
        //    }
        //}
        //match cpuid.get_extended_feature_info() {
        //    Some(info) => {
        //        warn!("Got CPU feature info {:?}", info);
        //    },
        //    None => {
        //        warn!("Couldn't get cpu feature info");
        //    }
        //}
        //match cpuid.get_performance_monitoring_info() {
        //    Some(info) => {
        //        warn!("Got CPU perfmonitoring info {:?}", info);
        //    },
        //    None => {
        //        warn!("Couldn't get cpu perfmonitoring info");
        //    }
        //}
        //match cpuid.get_thermal_power_info() {
        //    Some(info) => {
        //        warn!("Got CPU thermal info {:?}", info);
        //    },
        //    None => {
        //        warn!("Couldn't get cpu thermal info");
        //    }
        //}
        //match cpuid.get_extended_state_info() {
        //    Some(info) => {
        //        warn!("Got CPU state info {:?}", info);
        //    },
        //    None => {
        //        warn!("Couldn't get cpu state info");
        //    }
        //}
        //match cpuid.get_processor_capacity_feature_info() {
        //    Some(info) => {
        //        warn!("Got CPU capacity info {:?}", info);
        //    },
        //    None => {
        //        warn!("Couldn't get cpu capacity info");
        //    }
        //}
        //TODO: fix
        //i=0;
        //while i < nb_cpu_sockets {
        //    //topology.safe_add_domain_to_socket(i, , name, uj_counter, buffer_max_kbytes, sensor_data)
        //    i = i + 1;
        //}

        //topology.add_cpu_cores();
            
        for s in topology.get_sockets() {
            warn!("Inspecting CPUSocket: {:?}", s);
            unsafe {
                let core_id = s.get_cores_passive().last().unwrap().id + s.id * s.cpu_cores.len() as u16;
                warn!("Asking get_msr_value, from generate_tpopo, with core_id={}", core_id);
                match get_msr_value(core_id as usize, MSR_DRAM_ENERGY_STATUS as u64, &sensor_data) {
                    Ok(rec) => {
                        warn!("Added domain Dram !");
                        let mut domain_sensor_data = sensor_data.clone();
                        domain_sensor_data.insert(String::from("MSR_ADDR"), MSR_DRAM_ENERGY_STATUS.to_string());
                        domain_sensor_data.insert(String::from("CORE_ID"), core_id.to_string()); // nb of cores in a socket * socket_id + local_core_id
                        s.safe_add_domain(Domain::new(2, String::from("dram"), String::from(""), 5, domain_sensor_data))
                    },
                    Err(e) => {
                        error!("Could'nt add Dram domain.");
                    }
                }
                match get_msr_value(core_id as usize, MSR_PP0_ENERGY_STATUS as u64, &sensor_data) {
                    Ok(rec) => {
                        warn!("Added domain Core !");
                        let mut domain_sensor_data = sensor_data.clone();
                        domain_sensor_data.insert(String::from("MSR_ADDR"), MSR_PP0_ENERGY_STATUS.to_string());
                        domain_sensor_data.insert(String::from("CORE_ID"), core_id.to_string());
                        s.safe_add_domain(Domain::new(2, String::from("core"), String::from(""), 5, domain_sensor_data))
                    },
                    Err(e) => {
                        error!("Could'nt add Core domain.");
                    }
                }
                match get_msr_value(core_id as usize, MSR_PP1_ENERGY_STATUS as u64, &sensor_data) {
                    Ok(rec) => {
                        warn!("Added domain Uncore !");
                        let mut domain_sensor_data = sensor_data.clone();
                        domain_sensor_data.insert(String::from("MSR_ADDR"), MSR_PP1_ENERGY_STATUS.to_string());
                        domain_sensor_data.insert(String::from("CORE_ID"), core_id.to_string());
                        s.safe_add_domain(Domain::new(2, String::from("uncore"), String::from(""), 5, domain_sensor_data))
                    },
                    Err(e) => {
                        error!("Could'nt add Uncore domain.");
                    }
                }
                //match get_msr_value(core_id as usize, MSR_PLATFORM_ENERGY_STATUS as u64, &sensor_data) {
                //    Ok(rec) => {
                //    },
                //    Err(e) => {
                //        error!("Could'nt find Platform/PSYS domain.");
                //    }
                //}
            }
        }

        Ok(topology)
    }

    fn get_topology(&self) -> Box<Option<Topology>> {
        let topology = self.generate_topology().ok();
        if topology.is_none() {
            panic!("Couldn't generate the topology !");
        }
        Box::new(topology)
    }
}

unsafe fn get_msr_value(core_id: usize, msr_addr: u64, sensor_data: &HashMap<String, String>) -> Result<Record, String> {
    let current_process = GetCurrentProcess();
    let current_thread = GetCurrentThread();
    let mut thread_group_affinity = GROUP_AFFINITY { Mask: 255, Group: 9, Reserved: [0,0,0] };
    let thread_affinity_res  = GetThreadGroupAffinity(current_thread, &mut thread_group_affinity);
    if thread_affinity_res.as_bool() {
        warn!("Thread affinity found : {:?}", thread_group_affinity);
    } else {
        error!("Could'nt get thread group affinity");
    }
    let mut process_group_array: [u16; 8] = [0,0,0,0,0,0,0,0];
    let mut process_group_array_len = 8;
    let process_affinity_res = GetProcessGroupAffinity(current_process, &mut process_group_array_len, process_group_array.as_mut_ptr());
    if process_affinity_res.as_bool() {
        warn!("Process affinity found: {:?}", process_group_array);
    } else {
        error!("Could'nt get process group affinity");
        error!("Error was : {:?}", GetLastError());
    }
    warn!("Core ID requested to the driver : {}", core_id);
    match get_handle(sensor_data.get("DRIVER_NAME").unwrap()) {
        Ok(device) => {
            let mut msr_result: u64 = 0;
            let ptr_result = &mut msr_result as *mut u64;
            warn!("msr_addr: {:b}", msr_addr);
            warn!("core_id: {:x} {:b}", (core_id as u64), (core_id as u64));
            warn!("core_id: {:b}", ((core_id as u64) << 32));
            let src = ((core_id as u64) << 32) | msr_addr; //let src = ((core_id as u64) << 32) | msr_addr;
            let ptr = &src as *const u64;
        
            warn!("src: {:x}", src);
            warn!("src: {:b}", src);
            warn!("*ptr: {:b}", *ptr);
            //warn!("*ptr: {}", *ptr);
            //warn!("*ptr: {:b}", *ptr);

            match send_request(
                device,
                MSR_PKG_ENERGY_STATUS,
                ptr,
                8,
                ptr_result,
                size_of::<u64>(),
            ) {
                Ok(res) => {
                    close_handle(device);

                    let energy_unit = sensor_data
                        .get("ENERGY_UNIT")
                        .unwrap()
                        .parse::<f64>()
                        .unwrap();
                    let current_value = MsrRAPLSensor::extract_rapl_current_power(msr_result, energy_unit);
                    warn!("current_value: {}", current_value);

                    Ok(Record {
                        timestamp: current_system_time_since_epoch(),
                        unit: super::units::Unit::MicroJoule,
                        value: current_value,
                    })
                },
                Err(e) => {
                    error!("Failed to get data from send_request: {:?}", e);
                    close_handle(device);
                    Err(format!("Failed to get data from send_request: {:?}", e))
                }
            }
        },
        Err(e) => {
            error!("Couldn't get driver handle : {:?}", e);
            Err(format!("Couldn't get driver handle : {:?}", e))
        }
    }
}