#[cfg(feature = "disks_evaluation")]
use scaphandre::sensors::disk::{
    find_form_factor, find_physical_size, format_disk_name, DiskError, DiskState, FormFactor,
};
use scaphandre::sensors::units::Unit;
use std::collections::HashSet;

mod common;

#[test]
#[cfg(feature = "disks_evaluation")]
fn find_nvme_adapter() {
    common::setup_fs_nvme();
    let tmp_dir = common::tmp_tests_dir();
    let patterns = ["/dev/nvme0n1p3"];
    patterns.iter().for_each(|p| {
        let disk_name = format_disk_name(p);
        let driver = find_form_factor(&disk_name, tmp_dir.to_str().unwrap());
        assert_eq!(driver, FormFactor::NVME);
    });
}

#[test]
#[cfg(feature = "disks_evaluation")]
fn find_ahci_adapter() {
    common::setup_fs_scsi();
    let tmp_dir = common::tmp_tests_dir();
    let patterns = ["/dev/sda", "/dev/sda1", "/dev/sda1p1"];
    patterns.iter().for_each(|p| {
        let disk_name = format_disk_name(p);
        let driver = find_form_factor(&disk_name, tmp_dir.to_str().unwrap());
        assert_eq!(driver, FormFactor::SATA);
    });
}

#[test]
#[cfg(all(target_os = "linux", feature = "disks_evaluation"))]
fn it_should_find_the_physical_disk_size_of_a_disk_with_nvme_form_factor() {
    common::setup_fs_nvme();
    let tmp_dir = common::tmp_tests_dir();
    let disk_name = "nvme0n1";
    let disk_size = find_physical_size(disk_name, tmp_dir.to_str().unwrap());

    assert_eq!(disk_size.unwrap(), 512);
}

#[test]
#[cfg(all(target_os = "linux", feature = "disks_evaluation"))]
fn it_should_not_panick_if_no_sys_block_is_associated_to_a_disk_name() {
    common::setup_fs_nvme();
    let tmp_dir = common::tmp_tests_dir();
    let disk_name = "loop1";
    let attempt_to_find_disk_size = find_physical_size(disk_name, tmp_dir.to_str().unwrap());

    assert_eq!(
        attempt_to_find_disk_size.unwrap_err(),
        DiskError::NoBlockInSysfs
    );
}

#[test]
#[cfg(all(target_os = "linux", feature = "disks_evaluation"))]
fn it_should_add_an_evaluated_disk_to_the_topology() {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut mock_topology = common::generate_mock_topology(false);

    let mut disks_names = vec![];

    disks.iter().for_each(|disk| {
        let attempt_to_add_disk = mock_topology.add_sensor_disk(disk);
        match attempt_to_add_disk {
            Ok(_) => {
                let formatted_disk_name = format_disk_name(disk.name().to_str().unwrap());
                disks_names.push(formatted_disk_name)
            }
            Err(_) => println!("No disk added"),
        };
    });

    let unique_disks_names: Vec<String> = disks_names
        .clone()
        .into_iter()
        .map(|name| name)
        .collect::<HashSet<String>>()
        .into_iter()
        .collect();

    let topology_disks = mock_topology.disks.iter().enumerate();
    topology_disks.for_each(|entry| {
        let index = entry.0;
        let tdisk = entry.1;
        assert_eq!(tdisk.name, unique_disks_names[index]);
    });

    assert!(mock_topology.disks.len() >= 1);
    assert_eq!(mock_topology.disks.len(), unique_disks_names.len());
}

#[test]
#[cfg(all(target_os = "linux", feature = "disks_evaluation"))]
fn it_should_refresh_the_topology_disks_with_power_specs() {
    let mut mock_topology_with_disks = common::generate_mock_topology(true);
    let read_bytes = 1024;
    let written_bytes = 2048;
    mock_topology_with_disks.disks.iter_mut().for_each(|tdisk| {
        tdisk.refresh(read_bytes, written_bytes);
        assert_eq!(tdisk.power_specs.clone().unwrap().idle, 0.5);
        assert_eq!(tdisk.state, DiskState::ReadWrite);
        assert_eq!(tdisk.record_buffer[0].value, "5000000");
    });
}

#[test]
#[cfg(all(target_os = "linux", feature = "disks_evaluation"))]
fn it_should_refresh_all_the_topology_disks_through_sysinfo() {
    let mut mock_topology = common::generate_mock_topology(false);
    let mut sys_disks = sysinfo::Disks::new_with_refreshed_list();

    let mut disks_names = vec![];

    sys_disks.iter().for_each(|sdisk| {
        let attempt_to_add_disk = mock_topology.add_sensor_disk(sdisk);
        match attempt_to_add_disk {
            Ok(_) => {
                let formatted_disk_name = format_disk_name(sdisk.name().to_str().unwrap());
                disks_names.push(formatted_disk_name)
            }
            Err(_) => println!("No disk added"),
        };
    });

    let unique_disks_names: Vec<String> = disks_names
        .clone()
        .into_iter()
        .map(|name| name)
        .collect::<HashSet<String>>()
        .into_iter()
        .collect();

    sys_disks.refresh(false);

    mock_topology.refresh_disks(&sys_disks);

    assert!(mock_topology.disks.len() >= 1);
    assert_eq!(mock_topology.disks.len(), unique_disks_names.len());
}

#[test]
#[cfg(all(target_os = "linux", feature = "disks_evaluation"))]
fn it_should_return_an_error_if_disk_is_already_present_in_topology_or_not_found() {
    let mut mock_topology = common::generate_mock_topology(false);
    let sys_disks = sysinfo::Disks::new_with_refreshed_list();
    let mut disks_names: Vec<String> = vec![];

    sys_disks.iter().for_each(|sdisk| {
        let attempt_to_add_disk = mock_topology.add_sensor_disk(sdisk);
        match attempt_to_add_disk {
            Ok(_) => {
                let formatted_disk_name = format_disk_name(sdisk.name().to_str().unwrap());
                disks_names.push(formatted_disk_name);
                println!("Disk added")
            }
            Err(e) => {
                let formatted_disk_name = format_disk_name(sdisk.name().to_str().unwrap());
                if disks_names.contains(&String::from(formatted_disk_name)) {
                    assert_eq!(e, DiskError::DiskAlreadyPresent);
                } else {
                    assert_eq!(e, DiskError::NoBlockInSysfs)
                }
                println!("Error: {e}");
            }
        };
    });
}

#[test]
#[cfg(all(target_os = "linux", feature = "disks_evaluation"))]
fn it_should_add_disk_energy_records_to_the_topology_record_buffer() {
    use scaphandre::sensors::{Record, RecordReader};

    let mut mock_topology = common::generate_mock_topology(true);
    let two_seconds = std::time::Duration::new(2, 0);
    let four_seconds = std::time::Duration::new(4, 0);

    let first_power_record = Record {
        timestamp: two_seconds,
        unit: Unit::MicroWatt,
        value: String::from("5000000"),
    };
    let second_power_record = Record {
        timestamp: four_seconds,
        unit: Unit::MicroWatt,
        value: String::from("5000000"),
    };

    mock_topology.disks.iter_mut().for_each(|topology_disk| {
        topology_disk.record_buffer.push(first_power_record.clone());
        topology_disk
            .record_buffer
            .push(second_power_record.clone());
    });

    let topology_record = mock_topology.read_record().unwrap();
    assert_eq!(topology_record.value, String::from("40000000"));
    assert_eq!(topology_record.unit, Unit::MicroJoule);
}
