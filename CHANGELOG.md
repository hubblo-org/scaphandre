All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased](https://github.com/hubblo-org/scaphandre/commits/main)

This may be not up to date, please check main branch.

## [0.4.1](https://github.com/hubblo-org/scaphandre/releases/tag/v0.4.0)

### Changed

- Updated k8s-sync crate to 0.2.3 to get authentication by token feature

## [0.4.0](https://github.com/hubblo-org/scaphandre/releases/tag/v0.4.0)

### Added

- Riemann exporter now supports mTLS: [#103](https://github.com/hubblo-org/scaphandre/pull/103) thanks @uggla !
- `--containers` option, in prometheus exporter, tells scaphandre to add labels to metrics related to a docker container or a kubernetes pod, to make getting metrics of a distributed application easier: [#84](https://github.com/hubblo-org/scaphandre/pull/109) thanks @rossf7 for the tests, feedbacks, helm configuration and thanks @uggla for the reviews !
- stdout exporter now allows to choose the number of processes to watch, with the --process-number flag and to filter processes watched thanks to a regex, with the --regex-filter option: [#98](https://github.com/hubblo-org/scaphandre/pull/98), thanks @uggla !
- MetricGenerator includes timestamp in Metrics now : [#113](https://github.com/hubblo-org/scaphandre/pull/113)

### Fixed

- Added Cargo.lock to the repository: [#111](https://github.com/hubblo-org/scaphandre/issues/111)
- Ensured domains names are feteched properly in any case : [#114](https://github.com/hubblo-org/scaphandre/pull/114) thanks @PierreRust !

### Changed

- Manipulating flags as a Vec of clap::Arg instead of a HashMap of ExporterOption in exporters: [#100](https://github.com/hubblo-org/scaphandre/pull/100), thanks @uggla !
- Json and Stdout exporters are now using MetricGenerator as an inteface to get metrics properly : [#113](https://github.com/hubblo-org/scaphandre/pull/113)

## [0.3.0](https://github.com/hubblo-org/scaphandre/releases/tag/v0.3.0)

### Added

- New MetricGenerator and Metric structs and helper functions to make writing exporters easier. Riemann and Prometheus exporters now share the same code pattern: [#79](https://github.com/hubblo-org/scaphandre/pull/79) thanks @uggla !
- New [Warp10](https://warp10.io/) exporter ! [#76](https://github.com/hubblo-org/scaphandre/pull/76)
- Updated riemann_client and protobuf crates dependencies [#70](https://github.com/hubblo-org/scaphandre/pull/70/files) thanks @uggla !
- Successfully tested on AMD CPUs (AMD Ryzen 5 2600X): [#55](https://github.com/hubblo-org/scaphandre/issues/55) (requires a kernel 5.11 or later) thanks @barnumbirr and @kamaradclimber !
- Scaphandre can now be tested locally thanks to a docker-compose stack ! [#61](https://github.com/hubblo-org/scaphandre/pull/61) thanks @PierreRust !
- Added a CITATION file for references: [#95](https://github.com/hubblo-org/scaphandre/issues/95) thanks @tstrempel !

### Fixed

- Allowing scaphandre to run even if intel_rapl modules are not found: [#65](https://github.com/hubblo-org/scaphandre/pull/65) (needed to run scaphandre on AMD CPUs)
- Fixed typos and lacks in the documentation: [#81](https://github.com/hubblo-org/scaphandre/pull/81), [#77](https://github.com/hubblo-org/scaphandre/pull/77), [#80](https://github.com/hubblo-org/scaphandre/issues/80) thanks @pierreozoux, @LudovicRousseau, @maethor, @wallet77
- Moved documentation output to another [repository](https://github.com/hubblo-org/scaphandre-documentation): [#94](https://github.com/hubblo-org/scaphandre/pull/94) (documentation is now available here: [https://hubblo-org.github.io/scaphandre-documentation](https://hubblo-org.github.io/scaphandre-documentation))
- Json exporter has been refactored: [#87](https://github.com/hubblo-org/scaphandre/pull/87) thanks @jdrouet !

## [0.2.0](https://github.com/hubblo-org/scaphandre/releases/tag/v0.2.0)

### Added

- Docker image ([hubblo/scaphandre](https://hub.docker.com/r/hubblo/scaphandre)), ubuntu based: [#48](https://github.com/hubblo-org/scaphandre/pull/48), thanks @rossf7
- Helm chart to run scaphandre as a DaemonSet in a kubernetes cluster: [#72](https://github.com/hubblo-org/scaphandre/pull/72) - thanks @rossf7
- JsonExporter, to get metrics in JSON either in stdout or files: [#68](https://github.com/hubblo-org/scaphandre/pull/68) - thanks @wallet77
- RiemannExporter, to send metrics to [Riemann](http://riemann.io) monitoring tool: [#58](https://github.com/hubblo-org/scaphandre/pull/58) - thanks @uggla
- --qemu flag on PrometheusExporter, to add a "vmname" label to metrics related to processes that represent qemu-kvm virtual machines: [#41](https://github.com/hubblo-org/scaphandre/pull/41) - thanks @uggla
- Better documentation structure (based on [divio's documentation framework](https://documentation.divio.com/) and [mdbook](https://rust-lang.github.io/mdBook/)): [#45](https://github.com/hubblo-org/scaphandre/pull/45), result here:  [https://hubblo-org.github.io/scaphandre-documentation/](https://hubblo-org.github.io/scaphandre/)
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
