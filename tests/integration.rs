use scaphandre::exporters::qemu::QemuExporter;
use scaphandre::sensors::powercap_rapl::PowercapRAPLSensor;
use std::fs;

#[test]
fn exporter_qemu() {
    let sensor = PowercapRAPLSensor::new(1, 1, false);
    let mut exporter = QemuExporter::new(Box::new(sensor));
    let path = "/var/lib/libvirt/scaphandre/integration_tests";
    exporter.iteration(String::from(path));
    let content = fs::read_dir(path);
    assert_eq!(content.is_ok(), true);
}
