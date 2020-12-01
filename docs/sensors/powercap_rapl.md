# Powercap_rapl sensor

## Pre-requesites

If on a bare metal machine, this sensors needs the following modules to be present and running:
- intel_rapl_common
- rapl

Energy consumption data can be directly collected on a physical machine only.
To collect energy consumption on a virtual machine, please see the [virtio_bus sensor]() (and its buddy the [virtio_bus exporter]()).

If running in a Qemu/KVM virtual machine, and hypervisor host runs scaphandre with the [qemu exporter](../exporters/qemu.md), this sensor can be run with `--vm` flag to work without needed modules and rely on files that are exposed to it by the host's instance of the qemu exporter.

## Usage

To explicitely call the powercap_rapl sensor from the command line use:

    scaphandre -s powercap_rapl EXPORTER # EXPORTER being the exporter name you want to use

You can see arguments available from the cli for this sensors with:

    scaphandre -s powercap_rapl -h

If running in a virtual machine:

    scaphandre --vm -s powercap_rapl EXPORTER

Please refer to doc.rs code documentation for more details.

## Options available

- `sensor-buffer-per-socket-max-kB`: Maximum memory size allowed, in KiloBytes, for storing energy consumption for each socket
- `sensor-buffer-per-domain-max-kB`: Maximum memory size allowed, in KiloBytes, for storing energy consumption for each domain

## Environment variables

If in `--vm` mode, you want to read metrics from another path than the default `/var/scaphandre`, set env var `SCAPHANDRE_POWERCAP_PATH` with the desired path.