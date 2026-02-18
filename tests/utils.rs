use scaphandre::sensors::utils::{BlockDevicesDrivers, find_driver, format_disk_name};

mod common;

#[test]
fn find_nvme_driver() {
    common::setup_fs_nvme();
    let tmp_dir = common::tmp_tests_dir();
    let patterns = ["/dev/nvme0n1p3"];
    patterns.iter().for_each(|p| {
        let disk_name = format_disk_name(p);
        let driver = find_driver(&disk_name, tmp_dir.to_str().unwrap());
        assert_eq!(driver, BlockDevicesDrivers::NVME);
    });
}
