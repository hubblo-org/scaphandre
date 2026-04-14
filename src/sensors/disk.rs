use crate::sensors::units::Unit;
use crate::sensors::utils::current_system_time_since_epoch;
use crate::sensors::{Record, RecordGenerator, RecordReader};
use csv::Reader;
use regex::Regex;
use serde::Deserialize;
use std::{
    error::Error,
    fmt::{self, Display},
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    str::FromStr,
};
use sysinfo::DiskKind;

// Parsing the CSV file in data/disks at build-time, which will make the records available in the
// built binary.
include!(concat!(env!("OUT_DIR"), "/csv_records.rs"));

pub struct Attributes {
    pub name: String,
    pub kind: String,
    pub file_system: String,
    pub mount_point: String,
    pub removable: String,
}

#[derive(Debug, Clone)]
pub struct Metric {
    pub name: String,
    pub description: String,
    pub record: Record,
}

#[derive(Debug)]
pub struct Metrics {
    pub total_bytes: Metric,
    pub available_bytes: Metric,
}

pub struct DiskMetrics {
    pub attributes: Attributes,
    pub metrics: Metrics,
}

impl IntoIterator for Metrics {
    type Item = Metric;
    type IntoIter = MetricsIntoIterator;

    fn into_iter(self) -> Self::IntoIter {
        MetricsIntoIterator {
            metrics: self,
            index: 0,
        }
    }
}

pub struct MetricsIntoIterator {
    metrics: Metrics,
    index: usize,
}

impl Iterator for MetricsIntoIterator {
    type Item = Metric;
    fn next(&mut self) -> Option<Self::Item> {
        let result = match self.index {
            0 => self.metrics.total_bytes.clone(),
            1 => self.metrics.available_bytes.clone(),
            _ => return None,
        };
        self.index += 1;
        Some(result)
    }
}

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

#[derive(Debug, PartialEq)]
pub enum DiskError {
    NoBlockInSysfs,
    DiskAlreadyPresent,
}

impl fmt::Display for DiskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            DiskError::NoBlockInSysfs => write!(
                f,
                "No associated block directory has been found for this disk name in sysfs!"
            ),
            DiskError::DiskAlreadyPresent => write!(f, "Disk is already available in topology."),
        }
    }
}

/// sysinfo::Disk::DiskKind cannot be deserialized. Using this wrapper to allow parsing the disk power
/// specifications file.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub enum DiskKindWrapper {
    HDD,
    SSD,
    Unknown,
}

impl From<DiskKind> for DiskKindWrapper {
    fn from(disk_kind: DiskKind) -> Self {
        match disk_kind {
            DiskKind::HDD => DiskKindWrapper::HDD,
            DiskKind::SSD => DiskKindWrapper::SSD,
            DiskKind::Unknown(_) => DiskKindWrapper::Unknown,
        }
    }
}

impl Display for DiskKindWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            DiskKindWrapper::HDD => write!(f, "HDD"),
            DiskKindWrapper::SSD => write!(f, "SSD"),
            DiskKindWrapper::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PowerModel {
    pub disks: Vec<DiskRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct DiskPowerSpecs {
    pub name: String,
    pub manufacturer: String,
    pub capacity: u64,
    pub kind: DiskKindWrapper,
    pub form_factor: FormFactor,
    pub idle: f32,
    pub write: f32,
    pub read: f32,
    pub read_write: Option<f32>,
    pub read_bytes: u64,
    pub written_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct DiskRecord {
    pub name: String,
    pub manufacturer: String,
    pub form_factor: FormFactor,
    pub kind: DiskKindWrapper,
    pub capacity: u64,
    pub idle: f32,
    pub read: f32,
    pub write: f32,
    pub read_write: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct EvaluatedDisk {
    pub name: String,
    pub form_factor: FormFactor,
    pub capacity: u64,
    pub kind: DiskKindWrapper,
    pub power_specs: Option<DiskPowerSpecs>,
    pub record_buffer: Vec<Record>,
    pub max_buffer_size: u16,
    pub state: DiskState,
    pub power_model: Option<PowerModel>,
}

impl EvaluatedDisk {
    /// Creates a new EvaluatedDisk, a representation of a physical disk, with an empty record buffer. It
    /// can be updated throughout the execution of Scaphandre.
    pub fn new(disk_data: &sysinfo::Disk) -> Result<Self, DiskError> {
        let disk_name = format_disk_name(disk_data.name().to_str().unwrap());
        let disk_form_factor = find_form_factor(&disk_name, "/");
        let attempt_physical_size = find_physical_size(&disk_name, "/");
        let disk_kind = DiskKindWrapper::from(disk_data.kind());

        match attempt_physical_size {
            Ok(size) => Ok(EvaluatedDisk {
                name: disk_name,
                capacity: size,
                form_factor: disk_form_factor,
                kind: disk_kind,
                power_specs: None,
                record_buffer: vec![],
                max_buffer_size: 1,
                state: DiskState::Unknown,
                power_model: None,
            }),
            Err(_) => Err(DiskError::NoBlockInSysfs),
        }
    }

    /// Returns the idle, write and read power consumption for a given disk.
    pub fn find_power_specs(&self, power_model: &PowerModel) -> DiskRecord {
        let mut similar_disks: Vec<&DiskRecord> = power_model
            .disks
            .iter()
            .filter(|disk_pm| disk_pm.form_factor == self.form_factor && disk_pm.kind == self.kind)
            .collect();

        let similar_disks_by_capacity: Vec<&&DiskRecord> = similar_disks
            .iter()
            .filter(|disk_pm| disk_pm.capacity == self.capacity)
            .collect();

        match similar_disks_by_capacity.is_empty() {
            false => {
                let record = similar_disks_by_capacity[0];
                DiskRecord {
                    name: record.name.to_owned(),
                    manufacturer: record.manufacturer.to_owned(),
                    form_factor: record.form_factor,
                    kind: record.kind,
                    capacity: record.capacity,
                    idle: record.idle,
                    read: record.read,
                    write: record.write,
                    read_write: record.read_write,
                }
            }
            true => {
                info!("No similar disk by capacity identified, falling back on closest disk by capacity!");
                similar_disks.sort_by(|a, b| a.capacity.cmp(&b.capacity));
                let length = similar_disks.len();
                let smallest_disk = similar_disks[0];
                let largest_disk = similar_disks[length - 1];
                if self.capacity < smallest_disk.capacity {
                    smallest_disk.clone()
                } else {
                    largest_disk.clone()
                }
            }
        }
    }

    /// Allocate power specifications to the disk, through the disk power model.
    /// The read and written bytes for a given Disk can be identified with sysinfo::Disk::usage.
    pub fn set_power_specs(&mut self, read_bytes: u64, written_bytes: u64) {
        if let Some(model) = &self.power_model {
            let identified_record_for_disk = self.find_power_specs(model);

            let power_specs = DiskPowerSpecs {
                name: identified_record_for_disk.name,
                manufacturer: identified_record_for_disk.manufacturer,
                form_factor: identified_record_for_disk.form_factor,
                kind: identified_record_for_disk.kind,
                capacity: identified_record_for_disk.capacity,
                idle: identified_record_for_disk.idle,
                read: identified_record_for_disk.read,
                write: identified_record_for_disk.write,
                read_write: identified_record_for_disk.read_write,
                read_bytes,
                written_bytes,
            };

            self.power_specs = Some(power_specs);
        } else {
            info!("No power model yet set!")
        }
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
            DiskState::ReadWrite => {
                let power_specs = power_specs.unwrap();
                if let Some(rw_power_consumption) = power_specs.read_write {
                    Some(rw_power_consumption)
                } else {
                    Some(power_specs.write)
                }
            }
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
    pub fn generate_energy_record(&self, records: (&Record, &Record)) -> Option<Record> {
        let parsed_values = (
            records.0.value.parse::<f64>(),
            records.1.value.parse::<f64>(),
        );

        if let (Ok(earlier_value), Ok(later_value)) = parsed_values {
            let microwatts_sum = earlier_value + later_value;
            let time_diff = records.1.timestamp.as_secs_f64() - records.0.timestamp.as_secs_f64();
            let energy_consumed = microwatts_sum * time_diff;

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
            let previous_specs = specs.clone();
            self.set_power_specs(read_bytes, written_bytes);
            self.update_state(&previous_specs);
            let new_record = self.generate_power_record();
            self.add_record(new_record.unwrap());
        } else {
            info!("No previous power specification, continuing execution until specifications are found!");
        }
    }
}

impl RecordGenerator for EvaluatedDisk {
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
        self.record_buffer.to_vec()
    }
    fn refresh_record(&mut self) {}
}

#[derive(Debug)]
struct NoRecordError;

impl fmt::Display for NoRecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "No energy record were generated!")
    }
}
impl Error for NoRecordError {}

impl RecordReader for EvaluatedDisk {
    fn read_record(&self) -> Result<Record, Box<dyn Error>> {
        // To generate and read a disk energy record, the record buffer needs to hold at least two
        // power records.
        if self.record_buffer.len() >= 2 {
            let last_record = self.record_buffer.last().unwrap();
            let penultimate_record = &self.record_buffer[self.record_buffer.len() - 2];

            match self.generate_energy_record((penultimate_record, last_record)) {
                Some(record) => Ok(record),
                None => Err(Box::new(NoRecordError)),
            }
        } else {
            Err(Box::new(NoRecordError))
        }
    }
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

pub fn find_physical_size(disk_name: &str, path: &str) -> Result<u64, DiskError> {
    let path = PathBuf::from_str(path).unwrap();
    let formatted_disk_name = format_disk_name(disk_name);
    let disk_path = path.join("sys/block").join(formatted_disk_name);
    let attempt_size_file = File::open(disk_path.join("size").to_str().unwrap());
    match attempt_size_file {
        Ok(size) => {
            let mut size_file = size;
            let mut size_buffer = String::new();
            size_file.read_to_string(&mut size_buffer).unwrap();

            let number_of_sectors = size_buffer.trim_end().parse::<u64>().unwrap();

            let base: u64 = 10;

            let physical_size = (number_of_sectors * 512) / base.pow(9);

            Ok(physical_size)
        }
        Err(_) => Err(DiskError::NoBlockInSysfs),
    }
}

pub fn generate_power_model() -> PowerModel {
    let mut records = parse_csv();
    let mut disks_power: Vec<DiskRecord> = vec![];
    let iter = records.deserialize();

    iter.for_each(|record| {
        let disk_model: DiskRecord = record.unwrap();
        disks_power.push(disk_model);
    });

    PowerModel { disks: disks_power }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn generate_mock_evaluated_disk() -> EvaluatedDisk {
        let one_terabyte_in_gigabytes = 1024;
        let test_power_model = generate_power_model();

        EvaluatedDisk {
            name: String::from("/dev/nvme0"),
            capacity: one_terabyte_in_gigabytes,
            form_factor: FormFactor::NVME,
            kind: DiskKindWrapper::SSD,
            max_buffer_size: 1,
            power_specs: None,
            record_buffer: vec![],
            state: DiskState::Unknown,
            power_model: Some(test_power_model),
        }
    }

    fn generate_mock_evaluated_disk_record() -> DiskRecord {
        DiskRecord {
            name: String::from("Disk name"),
            manufacturer: String::from("Disk manufacturer"),
            capacity: 1024,
            kind: DiskKindWrapper::SSD,
            form_factor: FormFactor::NVME,
            idle: 0.5,
            read: 3.0,
            write: 5.0,
            read_write: Some(4.0),
        }
    }

    fn generate_mock_power_specs(record: &DiskRecord) -> DiskPowerSpecs {
        DiskPowerSpecs {
            name: record.name.to_owned(),
            manufacturer: record.manufacturer.to_owned(),
            capacity: record.capacity,
            kind: record.kind,
            form_factor: record.form_factor,
            idle: record.idle,
            read: record.read,
            write: record.write,
            read_write: Some(record.read_write.unwrap()),
            read_bytes: 1024,
            written_bytes: 0,
        }
    }

    fn generate_mock_power_record(duration: Duration, value: String) -> Record {
        Record {
            timestamp: duration,
            value,
            unit: Unit::MicroWatt,
        }
    }

    #[test]
    fn it_should_create_a_new_disk_from_sysinfo() {
        let disks = sysinfo::Disks::new_with_refreshed_list();

        disks.iter().for_each(|disk| {
            let scaph_disk = EvaluatedDisk::new(disk);

            match scaph_disk {
                Ok(d) => {
                    let formatted_name_from_sysinfo =
                        format_disk_name(disk.name().to_str().unwrap());
                    assert_eq!(d.name, formatted_name_from_sysinfo);
                }
                Err(e) => println!("Error : {e:?}"),
            }
        });
    }

    #[test]
    fn it_should_generate_the_power_specs_for_a_disk() {
        let mut disk = generate_mock_evaluated_disk();

        disk.set_power_specs(1024, 1024);

        let disk_power_specs = disk.power_specs.unwrap();

        assert_eq!(disk_power_specs.name, String::from("Disk name"));
        assert_eq!(
            disk_power_specs.manufacturer,
            String::from("Disk manufacturer")
        );
        assert_eq!(disk_power_specs.capacity, 1024);
        assert_eq!(disk_power_specs.idle, 0.5);
        assert_eq!(disk_power_specs.read, 3.0);
        assert_eq!(disk_power_specs.write, 5.0);
        assert_eq!(disk_power_specs.read_write, None);
    }

    #[test]
    fn it_should_generate_record_for_power_consumption_for_a_given_disk() {
        let disk_record = generate_mock_evaluated_disk_record();
        let power_specs = generate_mock_power_specs(&disk_record);
        let mut disk = generate_mock_evaluated_disk();
        disk.power_specs = Some(power_specs);
        disk.state = DiskState::Write;

        let scaph_disk_record = disk.generate_power_record().unwrap();
        assert_eq!(Some(scaph_disk_record.value), Some(String::from("5000000")));
    }

    #[test]
    fn it_should_generate_record_for_power_consumption_for_a_given_disk_in_read_write_state() {
        let disk_record = generate_mock_evaluated_disk_record();
        let power_specs = generate_mock_power_specs(&disk_record);
        let mut disk = generate_mock_evaluated_disk();
        disk.power_specs = Some(power_specs);
        disk.state = DiskState::ReadWrite;

        let scaph_disk_record = disk.generate_power_record().unwrap();
        assert_eq!(Some(scaph_disk_record.value), Some(String::from("4000000")));
    }

    #[test]
    fn it_should_generate_record_for_energy_consumption_for_a_given_disk() {
        let two_seconds = std::time::Duration::new(2, 0);
        let four_seconds = std::time::Duration::new(4, 0);
        let first_record = generate_mock_power_record(two_seconds, String::from("5000000"));
        let second_record = generate_mock_power_record(four_seconds, String::from("3000000"));
        let disk_record = generate_mock_evaluated_disk_record();
        let power_specs = generate_mock_power_specs(&disk_record);
        let mut disk = generate_mock_evaluated_disk();
        disk.power_specs = Some(power_specs);
        disk.record_buffer.push(first_record.clone());
        disk.record_buffer.push(second_record.clone());
        disk.state = DiskState::Write;

        let energy_record = disk
            .generate_energy_record((&first_record, &second_record))
            .unwrap();

        assert_eq!(Some(energy_record.value), Some(String::from("16000000")));
    }

    #[test]
    fn it_should_add_a_record_to_the_buffer_for_a_given_disk() {
        let disk_record = generate_mock_evaluated_disk_record();
        let power_specs = generate_mock_power_specs(&disk_record);
        let mut disk = generate_mock_evaluated_disk();
        disk.power_specs = Some(power_specs);
        disk.state = DiskState::Write;

        let scaph_disk_record = disk.generate_power_record().unwrap();
        disk.add_record(scaph_disk_record);

        assert_eq!(disk.record_buffer.len(), 1);
    }

    #[test]
    fn it_should_clean_old_records_from_the_buffer_for_a_given_disk() {
        let disk_record = generate_mock_evaluated_disk_record();
        let power_specs = generate_mock_power_specs(&disk_record);
        let mut disk = generate_mock_evaluated_disk();
        disk.power_specs = Some(power_specs);
        disk.state = DiskState::Write;

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
        let disk_record = generate_mock_evaluated_disk_record();
        let power_specs = generate_mock_power_specs(&disk_record);
        let mut disk = generate_mock_evaluated_disk();
        disk.power_specs = Some(power_specs);
        disk.state = DiskState::Write;

        let scaph_disk_record = disk.generate_power_record().unwrap();
        disk.add_record(scaph_disk_record.clone());
        disk.add_record(scaph_disk_record.clone());
        disk.add_record(scaph_disk_record);

        let records_copy = disk.get_records_passive();
        assert_eq!(records_copy.len(), 3);
    }

    #[test]
    fn it_should_format_the_storage_device_name() {
        let sysinfo_disk_name_nvme = "/dev/nvme0n1p3";

        let storage_device_name = format_disk_name(sysinfo_disk_name_nvme);

        assert_eq!(storage_device_name, "nvme0n1");

        let sysinfo_disk_name_scsi = "/dev/sda1";

        let storage_device_name = format_disk_name(sysinfo_disk_name_scsi);

        assert_eq!(storage_device_name, "sda");
    }

    #[test]
    fn it_should_give_a_power_estimation_for_a_given_disk_specifications() {
        let first_disk_record = generate_mock_evaluated_disk_record();
        let first_power_specs = generate_mock_power_specs(&first_disk_record);
        let mut second_disk_record = generate_mock_evaluated_disk_record();
        second_disk_record.capacity = 2048;
        let mut disk = generate_mock_evaluated_disk();
        disk.power_specs = Some(first_power_specs.clone());

        let power_model = PowerModel {
            disks: vec![first_disk_record.clone(), second_disk_record.clone()],
        };

        let disk_power_consumption = disk.find_power_specs(&power_model);
        assert_eq!(&disk_power_consumption.idle, &first_power_specs.idle);
        assert_eq!(&disk_power_consumption.write, &first_power_specs.write);
        assert_eq!(&disk_power_consumption.read, &first_power_specs.read);
    }

    #[test]
    fn it_should_return_the_current_disk_state() {
        let disk_record = generate_mock_evaluated_disk_record();
        let first_power_specs = generate_mock_power_specs(&disk_record);
        let mut second_power_specs = generate_mock_power_specs(&disk_record);
        second_power_specs.written_bytes = 1024;
        let mut disk = generate_mock_evaluated_disk();
        disk.power_specs = Some(first_power_specs.clone());

        disk.power_specs = Some(second_power_specs.clone());
        let disk_state = disk.evaluate_disk_state(&first_power_specs);

        assert_eq!(disk_state, DiskState::Write);

        let mut third_power_specs = generate_mock_power_specs(&disk_record);
        third_power_specs.read_bytes = 2048;
        third_power_specs.written_bytes = 1024;

        disk.power_specs = Some(third_power_specs.clone());
        let disk_state = disk.evaluate_disk_state(&second_power_specs);

        assert_eq!(disk_state, DiskState::Read);

        let mut fourth_power_specs = generate_mock_power_specs(&disk_record);
        fourth_power_specs.read_bytes = 4096;
        fourth_power_specs.written_bytes = 2048;

        disk.power_specs = Some(fourth_power_specs.clone());
        let disk_state = disk.evaluate_disk_state(&third_power_specs);

        assert_eq!(disk_state, DiskState::ReadWrite);

        let mut fifth_power_specs = generate_mock_power_specs(&disk_record);
        fifth_power_specs.read_bytes = 4096;
        fifth_power_specs.written_bytes = 2048;

        disk.power_specs = Some(fifth_power_specs.clone());
        let disk_state = disk.evaluate_disk_state(&fourth_power_specs);

        assert_eq!(disk_state, DiskState::Idle);
    }

    #[test]
    fn it_should_update_the_disk_state() {
        let disk_record = generate_mock_evaluated_disk_record();
        let mut first_power_specs = generate_mock_power_specs(&disk_record);
        first_power_specs.written_bytes = 0;
        let mut disk = generate_mock_evaluated_disk();
        disk.power_specs = Some(first_power_specs);
        let mut refreshed_power_specs = generate_mock_power_specs(&disk_record);
        refreshed_power_specs.read_bytes = 4096;
        refreshed_power_specs.written_bytes = 2048;

        disk.update_state(&refreshed_power_specs);
        assert_eq!(disk.state, DiskState::ReadWrite);
    }

    #[test]
    fn it_updates_the_disk_for_all_the_necessary_fields() {
        let disk_record = generate_mock_evaluated_disk_record();
        let mut first_power_specs = generate_mock_power_specs(&disk_record);
        first_power_specs.written_bytes = 0;
        let mut disk = generate_mock_evaluated_disk();
        disk.power_specs = Some(first_power_specs);

        let unmodified_read_bytes = 1024;
        let grown_written_bytes = 2048;

        disk.refresh(unmodified_read_bytes, grown_written_bytes);

        let first_report = &disk.record_buffer[0];

        assert_eq!(disk.record_buffer.len(), 1);
        assert_eq!(disk.state, DiskState::Write);
        assert_eq!(first_report.value, "5000000");
    }

    #[test]
    fn it_reads_the_latest_generated_energy_record_for_a_given_disk() {
        let disk_record = generate_mock_evaluated_disk_record();
        let mut first_power_specs = generate_mock_power_specs(&disk_record);
        first_power_specs.written_bytes = 0;

        let two_seconds = std::time::Duration::new(2, 0);
        let four_seconds = std::time::Duration::new(4, 0);
        let first_record = generate_mock_power_record(two_seconds, String::from("5000000.0"));
        let second_record = generate_mock_power_record(four_seconds, String::from("5000000.0"));

        let mut disk = generate_mock_evaluated_disk();
        disk.power_specs = Some(first_power_specs);
        disk.record_buffer.push(first_record);
        disk.record_buffer.push(second_record);

        let energy_record = disk.read_record().unwrap();

        assert_eq!(energy_record.value, String::from("20000000"));
    }

    #[test]
    fn it_returns_an_error_if_an_attempt_to_read_a_disk_energy_record_without_enough_power_records_is_made(
    ) {
        let disk = generate_mock_evaluated_disk();

        let attempt_to_read_record = disk.read_record();

        assert!(attempt_to_read_record.is_err());
    }
    #[test]
    fn it_generates_a_power_model_from_csv_records() {
        let power_model = generate_power_model();

        assert_eq!(power_model.disks.len(), 2);
        assert_eq!(power_model.disks[0].capacity, 512);
        assert_eq!(power_model.disks[1].capacity, 1024);
    }

    #[test]
    fn it_should_find_the_closest_power_specs_for_a_disk_if_no_similar_capacity_was_identified_in_the_power_model(
    ) {
        let mut disk = generate_mock_evaluated_disk();
        disk.capacity = 2048;
        let power_model = generate_power_model();

        let power_specs = disk.find_power_specs(&power_model);

        assert_eq!(power_specs.capacity, 1024);
        assert_eq!(power_specs.read, 3.0);
        assert_eq!(power_specs.write, 5.0);

        let mut smaller_disk = generate_mock_evaluated_disk();
        smaller_disk.capacity = 256;

        let power_specs = smaller_disk.find_power_specs(&power_model);
        assert_eq!(power_specs.capacity, 512);
        assert_eq!(power_specs.read, 3.0);
        assert_eq!(power_specs.write, 5.0);
    }
}
