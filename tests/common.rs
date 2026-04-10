use scaphandre::sensors::disk::{
    Disk, DiskKindWrapper, DiskPowerSpecs, DiskRecord, DiskState, FormFactor, PowerModel,
};
use scaphandre::sensors::utils::ProcessTracker;
use scaphandre::sensors::Topology;
use std::collections::HashMap;
use std::io::Write;
use std::{
    fs::{create_dir, create_dir_all, remove_dir_all},
    path::{Path, PathBuf},
};

pub fn tmp_tests_dir() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let tests_dir = Path::new(manifest_dir).join("tests");
    tests_dir.join("tmp")
}

pub fn setup_fs_nvme() {
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

    let nvme_ns_path = tmp_mock_block_path.join("nvme0n1");
    let size_file_path = nvme_ns_path.join("size");
    let mut size_file = std::fs::File::create_new(size_file_path).unwrap();
    size_file.write_all("1000215216".as_bytes()).unwrap();
    let _ = create_dir_all(nvme_ns_path.clone());

    let nvme_dev_path = nvme_ns_path.join("device/device");
    let _ = create_dir_all(nvme_dev_path.clone());

    let mock_driver_path = tmp_dir.join("sys/bus/pci/drivers/nvme");
    let _ = create_dir_all(mock_driver_path.clone());

    let driver_sl_path = nvme_dev_path.join("driver");
    let _ = std::os::unix::fs::symlink(mock_driver_path, driver_sl_path);
}

pub fn setup_fs_scsi() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let tests_dir = Path::new(manifest_dir).join("tests");
    let tmp_dir = tests_dir.join("tmp");

    let _ = remove_dir_all(tmp_dir.clone());

    create_dir(tmp_dir.clone()).unwrap();

    let mock_sys_block_path = "sys/block";
    let tmp_mock_block_path = tmp_dir.clone().join(mock_sys_block_path);
    let _ = create_dir_all(tmp_mock_block_path.clone());

    let block_paths = ["sda", "loop1", "loop2", "loop3"];
    block_paths.iter().for_each(|bp| {
        let p = tmp_mock_block_path.join(bp);
        let _ = create_dir(p);
    });

    let sda_path = tmp_mock_block_path.join("sda");
    let _ = create_dir_all(sda_path.clone());
    let scsi_device_path = sda_path.join("device");
    let mock_pci_bus_path = tmp_dir
        .join("sys/devices/pci0000:00/0000:00:1f.2/ata2/host1/target1:0:0/1:0:0:0/block/sda");
    let _ = create_dir_all(mock_pci_bus_path.clone());
    let _ = std::os::unix::fs::symlink(mock_pci_bus_path, scsi_device_path);

    let mock_driver_path = tmp_dir.join("sys/bus/pci/drivers/ahci");
    let _ = create_dir_all(mock_driver_path.clone());

    let mock_devices_path = tmp_dir.join("sys/bus/pci/devices/0000:00:1f.2");
    let _ = create_dir_all(mock_devices_path.clone());

    let mock_devices_driver_path = mock_devices_path.join("driver");

    let _ = std::os::unix::fs::symlink(mock_driver_path, mock_devices_driver_path);
}

pub fn generate_mock_topology(disks: bool) -> Topology {
    let mut mock_sensor_data = HashMap::new();
    mock_sensor_data.insert(String::from("key"), String::from("value"));
    let proc_tracker = ProcessTracker::new(5);

    if !disks {
        let mock_topology = Topology {
            sockets: vec![],
            stat_buffer: vec![],
            record_buffer: vec![],
            buffer_max_kbytes: 1,
            domains_names: None,
            _sensor_data: mock_sensor_data,
            proc_tracker,
            disks: vec![],
        };

        return mock_topology;
    } else {
        let power_specs = DiskPowerSpecs {
            name: String::from("Disk name"),
            manufacturer: String::from("Disk manufacturer"),
            kind: DiskKindWrapper::SSD,
            capacity: 1024,
            form_factor: FormFactor::NVME,
            idle: 0.5,
            read: 3.0,
            write: 5.0,
            read_write: None,
            read_bytes: 0,
            written_bytes: 0,
        };

        let power_model = generate_mock_power_model();

        let disk = Disk {
            name: String::from("/dev/nvme0"),
            form_factor: FormFactor::NVME,
            kind: DiskKindWrapper::SSD,
            capacity: 1024,
            max_buffer_size: 1,
            record_buffer: vec![],
            power_specs: Some(power_specs),
            state: DiskState::Unknown,
            power_model: Some(power_model),
        };

        let mock_topology = Topology {
            sockets: vec![],
            stat_buffer: vec![],
            record_buffer: vec![],
            buffer_max_kbytes: 1,
            domains_names: None,
            _sensor_data: mock_sensor_data,
            proc_tracker,
            disks: vec![disk.clone(), disk.clone()],
        };
        return mock_topology;
    }
}

// build.rs will create the disk records from the fixture available in tests/fixtures, but
// integration tests are built with source code, not test code. Using this method therefore to
// allocate a mocked power model to not rely on real disks available in the test environment.
pub fn generate_mock_power_model() -> PowerModel {
    let first_disk_record = DiskRecord {
        name: String::from("Disk name"),
        manufacturer: String::from("Disk manufacturer"),
        kind: DiskKindWrapper::SSD,
        capacity: 1024,
        form_factor: FormFactor::NVME,
        idle: 0.5,
        read: 3.0,
        write: 5.0,
        read_write: None,
    };
    let second_disk_record = DiskRecord {
        name: String::from("Disk name"),
        manufacturer: String::from("Disk manufacturer"),
        kind: DiskKindWrapper::SSD,
        capacity: 512,
        form_factor: FormFactor::NVME,
        idle: 0.5,
        read: 3.0,
        write: 5.0,
        read_write: None,
    };
    PowerModel {
        disks: vec![first_disk_record, second_disk_record],
    }
}
