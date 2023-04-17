//#[cfg(target_os = "linux")]
//use scaphandre::exporters::qemu::QemuExporter;
//#[cfg(target_os = "linux")]
//use scaphandre::sensors::powercap_rapl::PowercapRAPLSensor;
//use std::env::current_dir;
//use std::fs::{create_dir, read_dir};

//#[cfg(target_os = "linux")]
//#[test]
//fn exporter_qemu() {
//    use scaphandre::{exporters::{MetricGenerator, utils::get_hostname}, sensors::Sensor};
//
//    let sensor = PowercapRAPLSensor::new(1, 1, false);
//    let mut exporter = QemuExporter::new(Box::new(sensor));
//    // Create integration_tests directory if it does not exist
//    let curdir = current_dir().unwrap();
//    let path = curdir.join("integration_tests");
//    if !path.is_dir() {
//        create_dir(&path).expect("Fail to create integration_tests directory");
//    }
//    // Convert to std::string::String
//    let path = path.into_os_string().to_str().unwrap().to_string();
//    if let Ok(mut topology) = &sensor.generate_topology() {
//        let mut metric_generator = MetricGenerator::new(topology, get_hostname(), true, false);
//        exporter.iteration(path.clone(), &mut metric_generator);
//        let content = read_dir(path);
//        assert_eq!(content.is_ok(), true);
//    }
//}
//