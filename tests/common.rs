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

    let nvme_dev_path = tmp_mock_block_path.join("nvme0n1").join("device/device");
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
    let mock_pci_bus_path =
        tmp_dir.join("sys/devices/pci0000:00/0000:00:1f.2/ata2/host1/target1:0:0/1:0:0:0/block/sda");
    let _ = create_dir_all(mock_pci_bus_path.clone());
    let _ = std::os::unix::fs::symlink(mock_pci_bus_path, scsi_device_path);

    let mock_driver_path = tmp_dir.join("sys/bus/pci/drivers/ahci");
    let _ = create_dir_all(mock_driver_path.clone());

    let mock_devices_path = tmp_dir.join("sys/bus/pci/devices/0000:00:1f.2");
    let _ = create_dir_all(mock_devices_path.clone());

    let mock_devices_driver_path = mock_devices_path.join("driver");

    let _ = std::os::unix::fs::symlink(mock_driver_path, mock_devices_driver_path);
}
