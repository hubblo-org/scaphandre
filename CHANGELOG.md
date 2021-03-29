All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased](https://github.com/hubblo-org/scaphandre/commits/main)

This may be not up to date, please check main branch.

## [0.2.0](https://github.com/hubblo-org/scaphandre/releases/tag/v0.2.0)

### Added

- Docker image ([hubblo/scaphandre](https://hub.docker.com/r/hubblo/scaphandre)), ubuntu based: [#48](https://github.com/hubblo-org/scaphandre/pull/48), thanks @rossf7
- Helm chart to run scaphandre as a DaemonSet in a kubernetes cluster: [#72](https://github.com/hubblo-org/scaphandre/pull/72) - thanks @rossf7
- JsonExporter, to get metrics in JSON either in stdout or files: [#68](https://github.com/hubblo-org/scaphandre/pull/68) - thanks @wallet77
- RiemannExporter, to send metrics to [Riemann](http://riemann.io) monitoring tool: [#58](https://github.com/hubblo-org/scaphandre/pull/58) - thanks @uggla
- --qemu flag on PrometheusExporter, to add a "vmname" label to metrics related to processes that represent qemu-kvm virtual machines: [#41](https://github.com/hubblo-org/scaphandre/pull/41) - thanks @uggla
- Better documentation structure (based on [divio's documentation framework](https://documentation.divio.com/) and [mdbook](https://rust-lang.github.io/mdBook/)): [#45](https://github.com/hubblo-org/scaphandre/pull/45), result here:  [https://hubblo-org.github.io/scaphandre-documentation-documentation/](https://hubblo-org.github.io/scaphandre/)
- Automated CI tests including cargo test --all, running on a (bare metal) machine: [#62](https://github.com/hubblo-org/scaphandre/pull/62)

### Fixed

- Improved QemuExporter documentation: [#42](https://github.com/hubblo-org/scaphandre/pull/42) - thanks @uggla

## [0.1.1](https://github.com/hubblo-org/scaphandre/releases/tag/v0.1.1)

### Added

- `-s, --step` option added by @Uggla, to specify time step between two measurements, when using StdoutExporter

### Removed

- removed `energy_records_to_power_record` function `from src/sensors/mod.rs`. `get_records_diff_power_microwatts` functions from Topology and CPUSocket should be used instead

### Fixed

- README typos and misusage of english fixed by @Uggla and @florimondmanca
- @Uggla made init.sh script more robust
- cleaning of internal structs (records, cpustat, processes) was buggy, it is fixed
- linux kernels < 5, on intel CPUs (>2012) may me measurable now (fixed kernel modules names check)


## [0.1.0](https://github.com/hubblo-org/scaphandre/releases/tag/v0.1.0)

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
