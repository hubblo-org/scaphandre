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

    let block_paths = ["sda1", "loop1", "loop2", "loop3"];
    block_paths.iter().for_each(|bp| {
        let p = tmp_mock_block_path.join(bp);
        let _ = create_dir(p);
    });

    let scsi_dev_path = tmp_mock_block_path.join("sda1").join("device/device");
    let _ = create_dir_all(scsi_dev_path.clone());
    let mock_driver_path = tmp_dir.join("sys/bus/pci/drivers/sg");
    let _ = create_dir_all(mock_driver_path.clone());

    let driver_sl_path = scsi_dev_path.join("driver");
    let _ = std::os::unix::fs::symlink(mock_driver_path, driver_sl_path);
}
