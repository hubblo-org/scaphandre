use crate::sensors::units::Unit;
use crate::sensors::utils::current_system_time_since_epoch;
use crate::sensors::{Record, RecordGenerator};
use csv;
use regex::Regex;
use serde::Deserialize;
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq)]
pub enum DiskState {
    Idle,
    Write,
    Read,
    ReadWrite,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub enum FormFactor {
    NVME,
    SATA,
    Unknown,
}

impl std::fmt::Display for FormFactor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FormFactor::NVME => write!(f, "NVME"),
            FormFactor::SATA => write!(f, "SATA"),
            FormFactor::Unknown => write!(f, "Unknown form factor"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PowerModel {
    pub disks: Vec<DiskPowerSpecs>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct DiskPowerSpecs {
    pub capacity: u64,
    pub form_factor: FormFactor,
    pub idle: f32,
    pub write: f32,
    pub read: f32,
    pub read_bytes: u64,
    pub written_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct DiskPowerConsumption {
    pub form_factor: FormFactor,
    pub capacity: u64,
    pub idle: f32,
    pub read: f32,
    pub write: f32,
}

#[derive(Clone, Debug)]
pub struct Disk {
    pub name: String,
    pub form_factor: FormFactor,
    pub capacity: u64,
    pub power_specs: Option<DiskPowerSpecs>,
    pub record_buffer: Vec<Record>,
    pub max_buffer_size: u16,
    pub power_model_path: String,
    pub state: DiskState,
}

impl Disk {
    /// Creates a new Disk, with an empty record buffer, to be updated through the execution of
    /// Scaphandre.
    pub fn new(disk_data: &sysinfo::Disk) -> Self {
        let disk_name = String::from(disk_data.name().to_str().unwrap());
        let disk_form_factor = find_form_factor(&disk_name, "/");
        Disk {
            name: disk_name,
            capacity: disk_data.total_space(),
            form_factor: disk_form_factor,
            power_specs: None,
            record_buffer: vec![],
            max_buffer_size: 1,
            power_model_path: String::from("No power_model_path provided"),
            state: DiskState::Unknown,
        }
    }

    /// Returns the idle, write and read power consumption for a given disk.
    pub fn find_power_specs(&self, power_model: PowerModel) -> DiskPowerSpecs {
        let capacity_in_gigabytes = self.capacity / 1073741824;

        let similar_disks_by_form_factor: Vec<DiskPowerSpecs> = power_model
            .disks
            .into_iter()
            .filter(|disk_pm| disk_pm.form_factor == self.form_factor)
            .collect();

        let similar_disks_by_capacity: Vec<DiskPowerSpecs> = similar_disks_by_form_factor
            .into_iter()
            .filter(|disk_pm| disk_pm.capacity == capacity_in_gigabytes as u64)
            .collect();

        similar_disks_by_capacity[0].clone()
    }

    /// Attribute power specifications to the disk, by parsing a CSV file with the documented power
    /// model for various disk archetypes. The read and written bytes for a given Disk can be
    /// identified with sysinfo::Disk::usage.
    fn set_power_specs(&mut self, read_bytes: u64, written_bytes: u64, file_path: PathBuf) {
        let mut records = csv::Reader::from_path(file_path).unwrap();

        let mut disks_power: Vec<DiskPowerSpecs> = vec![];
        let mut iter = records.deserialize();

        if let Some(result) = iter.next() {
            let consumption: DiskPowerConsumption = result.unwrap();

            let power_specs = DiskPowerSpecs {
                form_factor: self.form_factor,
                capacity: self.capacity / 1073741824,
                idle: consumption.idle,
                read: consumption.read,
                write: consumption.write,
                read_bytes,
                written_bytes,
            };

            disks_power.push(power_specs);
        }

        let power_model = PowerModel { disks: disks_power };

        let identified_specs_for_disk = self.find_power_specs(power_model);

        self.power_specs = Some(identified_specs_for_disk);
    }

    /// Returns the disk's current state : idle, reading and / or writing.
    fn evaluate_disk_state(&self, previous_disk_specs: &DiskPowerSpecs) -> DiskState {
        let read_bytes_difference = self
            .power_specs
            .clone()
            .unwrap()
            .read_bytes
            .overflowing_sub(previous_disk_specs.read_bytes)
            .0;

        let written_bytes_difference = self
            .power_specs
            .clone()
            .unwrap()
            .written_bytes
            .overflowing_sub(previous_disk_specs.written_bytes)
            .0;

        let is_reading = !matches!(read_bytes_difference, 0);
        let is_writing = !matches!(written_bytes_difference, 0);
        match (is_reading, is_writing) {
            (false, true) => DiskState::Write,
            (true, false) => DiskState::Read,
            (true, true) => DiskState::ReadWrite,
            (false, false) => DiskState::Idle,
        }
    }

    fn update_state(&mut self, new_disk_specs: &DiskPowerSpecs) {
        let new_state = self.evaluate_disk_state(new_disk_specs);
        self.state = new_state;
    }

    /// Creates a record with the disk's power consumption at a given moment.
    fn generate_power_record(&self) -> Option<Record> {
        let power_specs = self.power_specs.clone();
        let consumption = match self.state {
            DiskState::Idle => Some(power_specs.unwrap().idle),
            DiskState::Read => Some(power_specs.unwrap().read),
            DiskState::Write => Some(power_specs.unwrap().write),
            // Associating ReadWrite state to writing operation consumption until finding a better
            // solution.
            DiskState::ReadWrite => Some(power_specs.unwrap().write),
            _ => None,
        };

        if let Some(power_consumption) = consumption {
            let converted_to_microwatts =
                Unit::to(power_consumption as f64, &Unit::Watt, &Unit::MicroWatt).unwrap();
            let record = Record {
                timestamp: current_system_time_since_epoch(),
                value: converted_to_microwatts.to_string(),
                unit: Unit::MicroWatt,
            };
            return Some(record);
        }
        None
    }

    /// Creates a record with the disk's energy consumption between two power records, during the
    /// execution of Scaphandre.
    fn generate_energy_record(records: (Record, Record)) -> Option<Record> {
        let parsed_values = (
            records.0.value.parse::<u64>(),
            records.1.value.parse::<u64>(),
        );

        if let (Ok(earlier_value), Ok(later_value)) = parsed_values {
            let microwatts_sum = earlier_value + later_value;
            let time_diff = records.1.timestamp.as_secs_f64() - records.0.timestamp.as_secs_f64();

            let energy_consumed = microwatts_sum as f64 * time_diff;

            return Some(Record {
                timestamp: current_system_time_since_epoch(),
                value: energy_consumed.to_string(),
                unit: Unit::MicroJoule,
            });
        }
        None
    }

    /// Utilitary method to add a Record to the Disk's record buffer.
    fn add_record(&mut self, record: Record) {
        if !self.record_buffer.is_empty() {
            self.clean_old_records();
        }
        self.record_buffer.push(record);
    }

    // Need to convert return type to Result to properly return a non-panicking error. It might be
    // useful not to make scaphandre panic if no specifications are identified for a given disk, as
    // the power model might be lacking until its development.
    pub fn refresh(&mut self, read_bytes: u64, written_bytes: u64) {
        if let Some(specs) = &self.power_specs {
            let power_model_path = PathBuf::from_str(&self.power_model_path).unwrap();
            let previous_specs = specs.clone();
            self.set_power_specs(read_bytes, written_bytes, power_model_path);
            self.update_state(&previous_specs);
            let new_record = self.generate_power_record();
            self.add_record(new_record.unwrap());
        } else {
            println!("No previous power specification, continuing execution until specifications are found!");
        }
    }
}

impl RecordGenerator for Disk {
    fn clean_old_records(&mut self) {
        let record_size = size_of_val(&self.record_buffer[0]) as u16;
        let current_size: u16 = (record_size * (self.record_buffer.len() as u16)) as u16;
        let max_size = self.max_buffer_size * 1024;
        if current_size >= max_size {
            let size_difference = current_size - max_size;
            if size_difference > record_size {
                let number_of_records_to_delete = size_difference / record_size;
                for _ in 1..number_of_records_to_delete {
                    if !self.record_buffer.is_empty() {
                        self.record_buffer.remove(0);
                    }
                }
            }
        }
    }
    fn get_records_passive(&self) -> Vec<Record> {
        let records_copy = self.record_buffer.to_vec();
        records_copy
    }
    fn refresh_record(&mut self) {}
}

/// The name for a disk identified through sysfs might include the partition number, the namespace
/// number for NVME devices, and other information. Only the device name is needed to find the driver.
pub fn format_disk_name(disk_path: &str) -> String {
    let disk_name = disk_path.split("/").last().unwrap();

    let device_name = match disk_name {
        // This gets the NVME controller and the namespace, useful to find the driver
        nvme_device if disk_name.starts_with("nvme") => {
            let pattern = Regex::new(r"nvme[0-9]n[0-9]").unwrap();
            let maybe_with_namespace = pattern.captures(nvme_device);
            match maybe_with_namespace {
                None => nvme_device.to_string(),
                Some(controller_and_namespace) => controller_and_namespace
                    .get(0)
                    .unwrap()
                    .as_str()
                    .to_string(),
            }
        }
        // Removing the partition number to only get the storage device name for SCSI block device
        scsi_device if disk_name.starts_with("sd") => {
            let v: Vec<&str> = scsi_device.split(char::is_numeric).collect();
            let device_name = v.first().unwrap().to_string();
            device_name
        }
        _ => String::from("Unknown"),
    };

    device_name
}

/// Return the form factor for a given stockage device, through driver identification.
pub fn find_form_factor(disk_name: &str, path: &str) -> FormFactor {
    let sys_block_path = PathBuf::from(path).join("sys/block");
    let disk_path = sys_block_path.join(disk_name);
    let disk_device_path = disk_path.join("device");

    let driver_name = match disk_name {
        _nvme_block_device if disk_name.starts_with("nvme") => {
            let try_driver = disk_device_path.join("driver").try_exists();

            match try_driver {
                Ok(true) => {
                    let driver_path = disk_device_path.join("driver").canonicalize().unwrap();

                    let driver_name = driver_path
                        .clone()
                        .to_str()
                        .unwrap()
                        .split("/")
                        .last()
                        .expect("Should return the last path part")
                        .to_string();
                    driver_name
                }
                Ok(false) => {
                    let parent_device_path = disk_device_path.join("device");
                    let driver_name = parent_device_path
                        .clone()
                        .join("driver")
                        .canonicalize()
                        .expect("Should resolve the driver symbolic link to the absolute path")
                        .to_str()
                        .expect("Should return a string")
                        .split("/")
                        .last()
                        .expect("Should return the last path part")
                        .to_string();
                    driver_name
                }
                Err(_) => String::from("Unknown path to driver"),
            }
        }
        _scsi_block_device if disk_name.starts_with("sd") => {
            let bus_node_resolved_link = disk_device_path
                .canonicalize()
                .expect("Should resolve the bus node path link to the absolute path");

            let bus_path = bus_node_resolved_link
                .to_str()
                .expect("Should return a string");

            let split_path: Vec<&str> = bus_path.split("/").collect();
            let bus_address_regex = Regex::new(r"[\w][\w][\w][\w]:[\w][\w]:[\w][\w]").unwrap();
            let find_bus_address: Vec<&&str> = split_path
                .iter()
                .filter(|path_section| bus_address_regex.is_match(path_section))
                .collect();

            let bus_address = find_bus_address.first().unwrap().to_string();

            let path_to_driver = PathBuf::from_str(path)
                .unwrap()
                .join("sys/bus/pci/devices")
                .join(bus_address)
                .join("driver");

            let resolve_symlink_to_driver = path_to_driver
                .canonicalize()
                .expect("Should resolve the bus driver symbolic link to the absolute path");

            let driver_path: Vec<&str> = resolve_symlink_to_driver
                .to_str()
                .unwrap()
                .split("/")
                .collect();

            let driver_name = driver_path.last().unwrap().to_string();

            driver_name
        }
        _ => String::from("Unknown block device"),
    };

    let adapter = match driver_name.as_str() {
        "nvme" => FormFactor::NVME,
        "ahci" => FormFactor::SATA,
        _ => FormFactor::Unknown,
    };

    adapter
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_should_create_a_new_disk_from_sysinfo() {
        let disks = sysinfo::Disks::new_with_refreshed_list();

        disks.iter().for_each(|disk| {
            let scaph_disk = Disk::new(disk);
            assert_eq!(scaph_disk.name, String::from(disk.name().to_str().unwrap()));
            assert_eq!(scaph_disk.capacity, disk.total_space());
        });
    }

    #[test]
    fn it_should_generate_the_power_specs_for_a_disk() {
        let cargo_manifest_dir = env!("CARGO_MANIFEST_DIR");
        let file_path = Path::new(cargo_manifest_dir).join("tests/fixtures/disk_power.csv");
        let ten_gigabytes_in_bytes = 1099511627776;

        let mut disk = Disk {
            name: String::from("/dev/nvme0"),
            capacity: ten_gigabytes_in_bytes,
            form_factor: FormFactor::NVME,
            max_buffer_size: 1,
            power_model_path: String::from("/"),
            power_specs: None,
            record_buffer: vec![],
            state: DiskState::Unknown,
        };

        disk.set_power_specs(1024, 1024, file_path);

        assert_eq!(disk.clone().power_specs.unwrap().capacity, 1024);
        assert_eq!(disk.clone().power_specs.unwrap().idle, 0.5);
        assert_eq!(disk.clone().power_specs.unwrap().read, 3.0);
        assert_eq!(disk.clone().power_specs.unwrap().write, 5.0);
    }

    #[test]
    fn it_should_generate_record_for_power_consumption_for_a_given_disk() {
        let ten_gigabytes_in_bytes = 1099511627776;
        let power_specs = DiskPowerSpecs {
            capacity: ten_gigabytes_in_bytes,
            form_factor: FormFactor::NVME,
            idle: 0.5,
            read: 3.0,
            write: 5.0,
            read_bytes: 1024,
            written_bytes: 2048,
        };

        let disk = Disk {
            name: String::from("/dev/nvme0"),
            capacity: ten_gigabytes_in_bytes,
            form_factor: FormFactor::NVME,
            max_buffer_size: 1,
            power_model_path: String::from("/"),
            power_specs: Some(power_specs),
            record_buffer: vec![],
            state: DiskState::Write,
        };

        let scaph_disk_record = disk.generate_power_record().unwrap();
        assert_eq!(Some(scaph_disk_record.value), Some(String::from("5000000")));
    }

    #[test]
    fn it_should_generate_record_for_energy_consumption_for_a_given_disk() {
        let two_seconds = std::time::Duration::new(2, 0);
        let four_seconds = std::time::Duration::new(4, 0);
        let first_record = Record {
            timestamp: two_seconds,
            value: String::from("5000000"),
            unit: Unit::MicroWatt,
        };

        let second_record = Record {
            timestamp: four_seconds,
            value: String::from("3000000"),
            unit: Unit::MicroWatt,
        };

        let energy_record = Disk::generate_energy_record((first_record, second_record)).unwrap();

        assert_eq!(Some(energy_record.value), Some(String::from("16000000")));
    }

    #[test]
    fn it_should_add_a_record_to_the_buffer_for_a_given_disk() {
        let ten_gigabytes_in_bytes = 1099511627776;

        let power_specs = DiskPowerSpecs {
            capacity: ten_gigabytes_in_bytes,
            form_factor: FormFactor::NVME,
            idle: 0.5,
            read: 3.0,
            write: 5.0,
            read_bytes: 1024,
            written_bytes: 2048,
        };
        let mut disk = Disk {
            name: String::from("/dev/nvme0"),
            capacity: ten_gigabytes_in_bytes,
            form_factor: FormFactor::NVME,
            max_buffer_size: 1,
            power_model_path: String::from("/"),
            power_specs: Some(power_specs),
            record_buffer: vec![],
            state: DiskState::Write,
        };

        let scaph_disk_record = disk.generate_power_record().unwrap();
        disk.add_record(scaph_disk_record);

        assert_eq!(disk.record_buffer.len(), 1);
    }

    #[test]
    fn it_should_clean_old_records_from_the_buffer_for_a_given_disk() {
        let ten_gigabytes_in_bytes = 1099511627776;

        let power_specs = DiskPowerSpecs {
            capacity: ten_gigabytes_in_bytes,
            form_factor: FormFactor::NVME,
            idle: 0.5,
            read: 3.0,
            write: 5.0,
            read_bytes: 1024,
            written_bytes: 2048,
        };
        let mut disk = Disk {
            name: String::from("/dev/nvme0"),
            capacity: ten_gigabytes_in_bytes,
            form_factor: FormFactor::NVME,
            max_buffer_size: 1,
            power_model_path: String::from("/"),
            power_specs: Some(power_specs),
            record_buffer: vec![],
            state: DiskState::Write,
        };

        let scaph_disk_record = disk.generate_power_record().unwrap();
        let max_size = 1024;
        let size_of_record = size_of_val(&scaph_disk_record);
        let max_number_of_records = max_size / size_of_record;

        while disk.record_buffer.len() != max_number_of_records {
            disk.add_record(scaph_disk_record.clone());
        }

        disk.clean_old_records();

        assert_eq!(disk.record_buffer.len(), max_number_of_records);
    }

    #[test]
    fn it_should_get_a_copy_of_a_disk_records() {
        let ten_gigabytes_in_bytes = 1099511627776;

        let power_specs = DiskPowerSpecs {
            capacity: ten_gigabytes_in_bytes,
            form_factor: FormFactor::NVME,
            idle: 0.5,
            read: 3.0,
            write: 5.0,
            read_bytes: 1024,
            written_bytes: 2048,
        };
        let mut disk = Disk {
            name: String::from("/dev/nvme0"),
            capacity: ten_gigabytes_in_bytes,
            form_factor: FormFactor::NVME,
            max_buffer_size: 1,
            power_model_path: String::from("/"),
            power_specs: Some(power_specs),
            state: DiskState::Write,
            record_buffer: vec![],
        };

        let scaph_disk_record = disk.generate_power_record().unwrap();
        disk.add_record(scaph_disk_record.clone());
        disk.add_record(scaph_disk_record.clone());
        disk.add_record(scaph_disk_record);

        let records_copy = disk.get_records_passive();
        assert_eq!(records_copy.len(), 3);
    }

    #[cfg(all(test, target_os = "linux"))]
    #[cfg(feature = "disks_evaluation")]
    #[test]
    fn it_should_format_the_storage_device_name() {
        use super::*;
        let sysinfo_disk_name_nvme = "/dev/nvme0n1p3";

        let storage_device_name = format_disk_name(sysinfo_disk_name_nvme);

        assert_eq!(storage_device_name, "nvme0n1");

        let sysinfo_disk_name_scsi = "/dev/sda1";

        let storage_device_name = format_disk_name(sysinfo_disk_name_scsi);

        assert_eq!(storage_device_name, "sda");
    }

    #[cfg(all(test, target_os = "linux"))]
    #[cfg(feature = "disks_evaluation")]
    #[test]
    fn it_should_identify_the_driver_for_nvme() {
        use super::*;
        use std::{
            fs::{create_dir, create_dir_all, remove_dir_all},
            path::Path,
        };

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let tests_dir = Path::new(manifest_dir).join("tests");
        let tmp_dir = tests_dir.join("tmp");

        let _ = remove_dir_all(tmp_dir.clone());

        create_dir(tmp_dir.clone()).unwrap();

        let mock_sys_block_path = "sys/block";
        let tmp_mock_block_path = tmp_dir.clone().join(mock_sys_block_path);
        let _ = create_dir_all(tmp_mock_block_path.clone());

        let block_paths = ["nvme0n1", "loop1", "loop2", "loop3"];
        block_paths.iter().for_each(|bp| {
            let p = tmp_mock_block_path.join(bp);
            let _ = create_dir(p);
        });

        let nvme_dev_path = tmp_mock_block_path.join("nvme0n1").join("device/device");
        let _ = create_dir_all(nvme_dev_path.clone());
        let mock_driver_path = tmp_dir.join("sys/bus/drivers/nvme");
        let _ = create_dir_all(mock_driver_path.clone());

        let driver_sl_path = nvme_dev_path.join("driver");
        let _ = std::os::unix::fs::symlink(mock_driver_path, driver_sl_path);

        let driver = find_form_factor("nvme0n1", tmp_dir.to_str().unwrap());

        assert_eq!(driver, FormFactor::NVME);
    }

    #[test]
    fn it_should_give_a_power_estimation_for_a_given_disk_specifications() {
        let disk_first_row = DiskPowerSpecs {
            capacity: 1024,
            form_factor: FormFactor::NVME,
            idle: 0.05,
            write: 8.0,
            read: 3.0,
            read_bytes: 0,
            written_bytes: 0,
        };
        let disk_second_row = DiskPowerSpecs {
            capacity: 2048,
            form_factor: FormFactor::SATA,
            idle: 0.8,
            write: 5.0,
            read: 2.0,
            read_bytes: 0,
            written_bytes: 0,
        };

        let disk = Disk {
            name: String::from("/dev/nvme0"),
            capacity: 1099511627776,
            form_factor: FormFactor::NVME,
            max_buffer_size: 1,
            power_model_path: String::from("/"),
            state: DiskState::Unknown,
            power_specs: Some(disk_first_row.clone()),
            record_buffer: vec![],
        };
        let power_model = PowerModel {
            disks: vec![disk_first_row.clone(), disk_second_row],
        };

        let disk_power_consumption = disk.find_power_specs(power_model);
        assert_eq!(disk_power_consumption.idle, disk_first_row.idle);
        assert_eq!(disk_power_consumption.write, disk_first_row.write);
        assert_eq!(disk_power_consumption.read, disk_first_row.read);
    }

    #[test]
    fn it_should_return_the_current_disk_state() {
        let first_disk_specs = DiskPowerSpecs {
            capacity: 1024,
            form_factor: FormFactor::NVME,
            idle: 0.05,
            write: 8.0,
            read: 3.0,
            read_bytes: 1024,
            written_bytes: 0,
        };

        let second_disk_specs = DiskPowerSpecs {
            capacity: 1024,
            form_factor: FormFactor::NVME,
            idle: 0.05,
            write: 8.0,
            read: 3.0,
            read_bytes: 1024,
            written_bytes: 1024,
        };

        let mut disk = Disk {
            name: String::from("/dev/nvme0"),
            capacity: 109951162776,
            form_factor: FormFactor::NVME,
            max_buffer_size: 1,
            power_model_path: String::from("/"),
            power_specs: Some(first_disk_specs.clone()),
            record_buffer: vec![],
            state: DiskState::Unknown,
        };

        disk.power_specs = Some(second_disk_specs.clone());
        let disk_state = disk.evaluate_disk_state(&first_disk_specs);

        assert_eq!(disk_state, DiskState::Write);

        let third_disk_specs = DiskPowerSpecs {
            capacity: 1024,
            form_factor: FormFactor::NVME,
            idle: 0.05,
            write: 8.0,
            read: 3.0,
            read_bytes: 2048,
            written_bytes: 1024,
        };

        disk.power_specs = Some(third_disk_specs.clone());
        let disk_state = disk.evaluate_disk_state(&second_disk_specs);

        assert_eq!(disk_state, DiskState::Read);

        let fourth_disk_specs = DiskPowerSpecs {
            capacity: 1024,
            form_factor: FormFactor::NVME,
            idle: 0.05,
            write: 8.0,
            read: 3.0,
            read_bytes: 4096,
            written_bytes: 2048,
        };

        disk.power_specs = Some(fourth_disk_specs.clone());
        let disk_state = disk.evaluate_disk_state(&third_disk_specs);

        assert_eq!(disk_state, DiskState::ReadWrite);

        let fifth_disk_specs = DiskPowerSpecs {
            capacity: 1024,
            form_factor: FormFactor::NVME,
            idle: 0.05,
            write: 8.0,
            read: 3.0,
            read_bytes: 4096,
            written_bytes: 2048,
        };

        disk.power_specs = Some(fifth_disk_specs.clone());
        let disk_state = disk.evaluate_disk_state(&fourth_disk_specs);

        assert_eq!(disk_state, DiskState::Idle);
    }

    #[test]
    fn it_should_update_the_disk_state() {
        let first_disk_specs = DiskPowerSpecs {
            capacity: 1024,
            form_factor: FormFactor::NVME,
            idle: 0.05,
            write: 8.0,
            read: 3.0,
            read_bytes: 1024,
            written_bytes: 0,
        };

        let mut disk = Disk {
            name: String::from("/dev/nvme0"),
            capacity: 109951162776,
            form_factor: FormFactor::NVME,
            max_buffer_size: 1,
            power_model_path: String::from("/"),
            power_specs: Some(first_disk_specs.clone()),
            record_buffer: vec![],
            state: DiskState::Unknown,
        };

        let refreshed_disk_specs = DiskPowerSpecs {
            capacity: 1024,
            form_factor: FormFactor::NVME,
            idle: 0.05,
            write: 8.0,
            read: 3.0,
            read_bytes: 4096,
            written_bytes: 2048,
        };

        disk.update_state(&refreshed_disk_specs);
        assert_eq!(disk.state, DiskState::ReadWrite);
    }

    #[test]
    fn it_updates_the_disk_for_all_the_necessary_fields() {
        let cargo_manifest_dir = env!("CARGO_MANIFEST_DIR");

        let file_path = Path::new(cargo_manifest_dir).join("tests/fixtures/disk_power.csv");
        let first_disk_specs = DiskPowerSpecs {
            capacity: 1024,
            form_factor: FormFactor::NVME,
            idle: 0.05,
            write: 8.0,
            read: 3.0,
            read_bytes: 1024,
            written_bytes: 0,
        };

        let mut disk = Disk {
            name: String::from("/dev/nvme0"),
            capacity: 109951162776,
            form_factor: FormFactor::NVME,
            max_buffer_size: 1,
            power_model_path: String::from(file_path.to_str().unwrap()),
            power_specs: Some(first_disk_specs.clone()),
            record_buffer: vec![],
            state: DiskState::Unknown,
        };

        let unmodified_read_bytes = 1024;
        let grown_written_bytes = 2048;

        disk.refresh(unmodified_read_bytes, grown_written_bytes);

        let first_report = &disk.record_buffer[0];

        assert_eq!(disk.record_buffer.len(), 1);
        assert_eq!(disk.state, DiskState::Write);
        assert_eq!(first_report.value, "5000000");
    }
}
