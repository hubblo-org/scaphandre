# Powercap_rapl sensor

## Pre-requesites

At the time those lines are written, this sensor works only on:

- OS: GNU/Linux
- Intel and AMD x86 CPUs, produced after 2012 (or some laptop cpu prior to 2012)

It needs the following kernel modules to be present and running:

On kernels 5.0 or later: `intel_rapl_common`

On kernel prior 5.0: `intel_rapl`

For AMD processors, it seems that powercap/rapl [will work only since kernel 5.8](https://www.phoronix.com/scan.php?page=news_item&px=Google-Zen-RAPL-PowerCap)
and [5.11 for family 19h](https://www.phoronix.com/scan.php?page=news_item&px=AMD-RAPL-Linux-Now-19h).

Energy consumption data can be directly collected on a **physical machine** only.

To collect energy consumption on a virtual machine, you may first collect power consumption data from the hypervisor thanks to the [qemu exporter](exporter-qemu.md) and then collect those metrics in the virtual machine thanks to this sensor, with `--vm` flag enabled.

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

## Troubleshooting

### When running scaphandre on Ubuntu 20.xx I get a `permission denied` error

Since linux kernel package 5.4.0-53.59 in debian/ubuntu, powercap attributes are only accessible by root:

    linux (5.4.0-53.59) focal; urgency=medium

      * CVE-2020-8694
        - powercap: make attributes only readable by root

Therefor, the user running scaphandre needs to have read access to *energy_uj* files in `/sys/class/powercap`.

You can run the [init.sh](../../init.sh) script to apply appropriate permissions to the required files.
