[Introduction](README.md)

# Tutorials

- [Getting Started](tutorials/getting_started.md)
- [Installation on GNU/Linux](tutorials/installation-linux.md)
- [Installation on Windows](tutorials/installation-windows.md)
- [Docker-compose](tutorials/docker-compose.md)
- [Compilation for GNU/Linux](tutorials/compilation-linux.md)
- [Compilation for Windows](tutorials/compilation-windows.md)
- [Power consumption of a Kubernetes cluster with scaphandre, prometheus and grafana](tutorials/kubernetes.md)

# How-to guides

- [Propagate power consumption metrics from hypervisor to virtual machines (Qemu/KVM)](how-to_guides/propagate-metrics-hypervisor-to-vm_qemu-kvm.md)
- [Get process-level power consumption in my grafana dashboard](how-to_guides/get-process-level-power-in-grafana.md)
- [Install Scaphandre with only Prometheus-push exporter compiled, for Prometheus Push Gateway, on RHEL 8 and 9](how-to_guides/install-prometheuspush-only-rhel.md)

# Explanations

- [Explanations about host level power and energy metrics](explanations/host_metrics.md)
- [How scaphandre computes per process power consumption](explanations/how-scaph-computes-per-process-power-consumption.md)
- [Internal structure](explanations/internal-structure.md)
- [About containers](explanations/about-containers.md)
- [About RAPL domains](explanations/rapl-domains.md)

# References

- [Metrics available](references/metrics.md)

## Exporters

- [JSON exporter](references/exporter-json.md)
- [Prometheus exporter](references/exporter-prometheus.md)
- [Prometheus-push exporter](references/exporter-prometheuspush.md)
- [Qemu exporter](references/exporter-qemu.md)
- [Riemann exporter](references/exporter-riemann.md)
- [Stdout exporter](references/exporter-stdout.md)
- [Warp10 exporter](references/exporter-warp10.md)

## Sensors

- [MSR_RAPL sensor](references/sensor-msr_rapl.md)
- [PowercapRAPL sensor](references/sensor-powercap_rapl.md)
- [MSRRAPL sensor](references/sensor-msr_rapl.md)

[Why this project ?](why.md)
[Compatibility](compatibility.md)
[Troubleshooting](troubleshooting.md)
[Contributing guide](contributing.md)
[External references you may be interested in](sources.md)
