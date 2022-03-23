# Internal structure

Scaphandre is designed to be extensible. As it performs basically two tasks: **collecting**/pre-computing the power consumption metrics and **publishing** it, it is composed of two main components: a **sensor** and an **exporter**. Each can be implemented in different ways to match a certain use case. When you run scaphandre from the command line, `-s` allows you to choose the sensor you want to use, and the next subcommand is the name of the exporter.

## Sensors

Sensors are meant to:

1. get the power consumptions metrics of the host
2. make it available for the exporter

The [PowercapRAPL](../references/sensor-powercap_rapl.md) for instance, gets and transforms metrics coming from the powercap Linux kernel module, that serves as an interface to get the data from the [RAPL](https://01.org/blogs/2014/running-average-power-limit-%E2%80%93-rapl) feature of x86 CPUs. Because this feature is only accessible when you are running on a bare metal machine, this sensor will not work in a virtual machine, except if you first run scaphandre on the hypervisor and make the VM metrics available, with the [qemu exporter](../references/exporter-qemu.md), to scaphandre running inside the virtual machine.

When you don't have access to the hypervisor/bare-metal machine (ie. when you run on public cloud instances and your provider doesn't run scaphandre) you still have the option to estimate the power consumption, based on both the ressources (cpu/gpu/ram/io...) consumed by the virtual machine at a given time, and the characteristics of the underlying hardware. This is the way we are designing the future [estimation-based sensor](https://github.com/hubblo-org/scaphandre/issues/25), to match that use case.

Looking at the code, you'll find that the interface between metrics and the exporters is in fact the [Topology](https://docs.rs/scaphandre/0.1.1/scaphandre/sensors/struct.Topology.html) object. This is intended to be asked by the exporter through the [get_topology](https://docs.rs/scaphandre/0.1.1/scaphandre/sensors/trait.Sensor.html#tymethod.get_topology) method of the sensor.

## Exporters

An exporter is expected to:

1. ask the sensors to get new metrics and store them for later, potential usage
2. export the current metrics 

The [Stdout](../references/exporter-stdout.md) exporter exposes the metrics on the standard output (in your terminal). The [prometheus](../references/exporter-prometheus.md) exporter exposes the metrics on an HTTP endpoint, to be scraped by a [prometheus](https://prometheus.io) instance. An exporter should be created for each monitoring scenario (do you want to feed your favorite monitoring/data analysis tool with scaphandre metrics ? feel free to open a [PR](https://github.com/hubblo-org/scaphandre/pulls) to create a new exporter !).

As introduced in the [sensors](#sensors) section, the [Qemu](../references/exporter-qemu.md) exporter, is very specific. It is only intended to collect metrics related to running virtual machines on a Qemu/KVM hypervisor. Those metrics can then be made available to each virtual machine and their own scaphandre instance, running the [PowercapRAPL](../references/sensor-powercap_rapl.md) sensor (with the `--vm` flag on). The qemu exporter puts VM's metrics in files the same way the powercap kernel module does it. It mimics this behavior, so the sensor can act the same way it would on a bare metal machine.