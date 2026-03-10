#[cfg(feature = "disks_evaluation")]
use scaphandre::sensors::utils::{find_form_factor, format_disk_name, FormFactor};

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
