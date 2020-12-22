# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1]

### Added

- `-s, --step` option added by @Uggla, to specify time step between two measurements, when using StdoutExporter

### Removed

- removed `energy_records_to_power_record` function `from src/sensors/mod.rs`. `get_records_diff_power_microwatts` functions from Topology and CPUSocket should be used instead

### Fixed

- README typos and misusage of english fixed by @Uggla and @florimondmanca
- @Uggla made init.sh script more robust
- cleaning of internal structs (records, cpustat, processes) was buggy, it is fixed
- linux kernels < 5, on intel CPUs (>2012) may me measurable now (fixed kernel modules names check)


## [0.1.0]

### Added

- Exporters and sensors design.
- Stdout exporter.
- Prometheus exporter.
- Powercap_rapl sensor.
- Qemu exporter.

## HELP

Here are the sub-sections that can be included in each release section:

- **Added** for new features.
- **Changed** for changes in existing functionality.
- **Deprecated** for soon-to-be removed features.
- **Removed** for now removed features.
- **Fixed** for any bug fixes.
- **Security** in case of vulnerabilities.
