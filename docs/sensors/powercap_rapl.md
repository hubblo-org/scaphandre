# Powercap_rapl sensor

## Pre-requesites

This sensors needs the following modules to be installed and running:
- intel_rapl_common
- rapl

Energy consumption data can be directly collected on a physical machine only.
To collect energy consumption on a virtual machine, please see the [virtio_bus sensor]() (and its buddy the [virtio_bus exporter]()).

## Usage

To explicitely call the powercap_rapl sensor from the command line use:

    scaphandre -s powercap_rapl EXPORTER # EXPORTER being the exporter name you want to use

You can see arguments available from the cli for this sensors with:

    scaphandre -s powercap_rapl -h

From the code, basic usage looks like:

    use scaphandre::sensors::PowercapRAPLSensor;

    let sensor = PowercapRAPLSensor::new(1, 1);
    # let's say you want to instantiate ChoosenExporter, which doesn't really exist
    ChoosenExporter::new(Box::new(sensor)).run();

Please refer to doc.rs code documentation for more details.

## Options available

- `sensor-buffer-per-socket-max-kB`: Maximum memory size allowed, in KiloBytes, for storing energy consumption for each socket
- `sensor-buffer-per-domain-max-kB`: Maximum memory size allowed, in KiloBytes, for storing energy consumption for each domain