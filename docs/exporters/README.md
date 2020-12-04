# Exporters documentation

- [Prometheus Exporter](prometheus.md): Exposes power consumption metrics of the host as a Prometheus compatible HTTP endpoint (also called exporter in Prometheus terminology)
- [Qemu Exporter](qemu.md): Looks for Qemu/KVM virtual machines on the host and keeps their energy consumption metrics in `/var/lib/libvirt/scaphandre/VN_NAME` (default)
- [Stdout Exporter](stdout.md): Shows power consumption metrics of the host via the standard output in the terminal