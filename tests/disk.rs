#[cfg(feature = "disks_evaluation")]
use scaphandre::sensors::disk::{find_form_factor, format_disk_name, DiskState, FormFactor};

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
fn it_should_add_an_evaluated_disk_to_the_topology() {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut mock_topology = common::generate_mock_topology(false);

    let mut disks_names = vec![];

    disks.iter().for_each(|disk| {
        mock_topology.add_sensor_disk(disk);
        disks_names.push(disk.name().to_str().unwrap());
    });

    let topology_disks = mock_topology.disks.iter().enumerate();
    topology_disks.for_each(|entry| {
        let index = entry.0;
        let tdisk = entry.1;
        assert_eq!(tdisk.name, disks_names[index]);
    });

    assert_eq!(mock_topology.disks.len(), disks.len());
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
    let mock_power_model_path = common::mock_power_model_path();
    let mut mock_topology = common::generate_mock_topology(false);
    let mut sys_disks = sysinfo::Disks::new_with_refreshed_list();
    sys_disks.iter().for_each(|sdisk| {
        mock_topology.add_sensor_disk(sdisk);
    });

    mock_topology.disks.iter_mut().for_each(|tdisk| {
        tdisk.power_model_path = String::from(mock_power_model_path.to_str().unwrap())
    });

    sys_disks.refresh(false);

    mock_topology.refresh_disks(&sys_disks);

    assert_eq!(mock_topology.disks.len(), sys_disks.len());
}
