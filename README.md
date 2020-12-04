# Scaphandre

<img src="https://github.com/hubblo-org/scaphandre/raw/main/scaphandre.cleaned.png" width="200">

---

Scaphandre is a metrology agent dedicated to electrical [power](https://en.wikipedia.org/wiki/Electric_power) consumption metrics. The goal of the project is to permit to any company or individual to measure the power consumption of its tech services and get those data in a convenient form, sending it though any monitoring or data analysis toolchain.

**Scaphandre** means *heavy* **diving suit** in [:fr:](https://fr.wikipedia.org/wiki/Scaphandre_%C3%A0_casque). It comes from the idea that tech related services often don't track their power consumption and thus don't expose it to their clients. Most of the time the reason is a presumed bad [ROI](https://en.wikipedia.org/wiki/Return_on_investment). Scaphandre makes, for tech providers and tech users, easier and cheaper to go under the surface to bring back the desired power consumption metrics, take better sustainability focussed decisions, and then show the metrics to their clients to allow them to do the same.

See the [why](docs/why.md) section for more about the goals of the project.

![Fmt+Clippy](https://github.com/hubblo-org/scaphandre/workflows/Rust/badge.svg?branch=main)
<a href="https://gitter.im/hubblo-org/scaphandre?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge"><img src="https://badges.gitter.im/Join%20Chat.svg"></a>

---

## Features

- measuring power consumption on bare metal hosts
- measuring power consumption on qemu/kvm virtual machines
- measuring power consumption of qemu/kvm virtual machines from the host
- exposing power consumption metrics as a [prometheus](https://prometheus.io) HTTP exporter

## Getting started

Installation steps are described [here](#Installation).

Here are some examples.

To show power consumption metrics in your terminal, run:

    scaphandre stdout

To expose power consumption metrics as a [prometheus](https://prometheus.io) exporter (as an http endpoint):

    scaphandre prometheus

Metrics are now available on http://localhost:8080/metrics.

To compute metrics of running Qemu/KVM virtual machines on the host, and [be able to expose those metrics](docs/exporters/qemu.md) to the guests, run:

    scaphandre qemu

General usage is:

    scaphandre [-s SENSOR] EXPORTER [OPTIONS]

Available exporters are:

- [stdout](docs/exporters/stdout.md): displays metrics on the standard output/on your terminal
- [prometheus](docs/exporters/prometheus.md): exposes metrics as an http endpoint, respecting the [prometheus](https://prometheus.io/) metrics standard
- [qemu](docs/exporters/qemu.md): computes power consumption of each Qemu/KVM virtual machine running on the host and stores the data in `/var/lib/libvirt/scaphandre/VM_NAME`

Available sensors are:

- [powercap_rapl](docs/sensors/powercap_rapl.md)

## Installation

You'll find existing releases and packages [here](https://github.com/hubblo-org/scaphandre/releases).

To hack *scaph*, or simply be up to date with latest developments, you can download scaphandre from the main branch:

    git clone https://github.com/hubblo-org/scaphandre.git
    cd scaphandre
    cargo build # binary path is target/debug/scaphandre

To use the latest code for a true use case, build for release instead of debug:

    cargo build --release
    
Binary path is `target/release/scaphandre`.

## Virtual Machines & Cloud

A major pain point in measuring power consumption is doing so inside a virtual machine. A virtual machine usually doesn't have access to power metrics.
Scaphandre aims at solving that by enabling a communication between a scaphandre instance on the hypervisor host and another one in the virtual machine.
The scaphandre agent on the host will compute the metrics meaningfull for that virtual machine and the one on the VM exploit those metrics to allow its user to exploit the data as if he had access to power metrics in the first place (as if he was on a bare metal machine).

This allows to break opacity in a virtualization, if you have access to the virtualization hosts and can install this tool, or cloud context if the provider uses scaphandre on his hypervisors. Please refer to the [qemu exporter](docs/exporters/qemu.md) documentation.

<img src="https://github.com/hubblo-org/scaphandre/raw/main/virtu.cleaned.png" width="600">

## Contributing

Feel free to propose pull requests, or open new issues at will. Scaphandre is a collaborative project and all opinions and propositions shall be heard and studied. The contributions will be received with kindness, gratitude and with an open mind. Remember that we are all dwarfs standing on the shoulders of giants. We all have to learn from others and to give back, with due mutual respect.

Discussions and questions about the project are welcome on gitter: [gitter](https://gitter.im/hubblo-org/scaphandre?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge) or by [email](mailto://bpetit@hubblo.org?Subject=About%20Scaphandre).

Here is the [code of conduct](CODE_OF_CONDUCT.md) of the project.

This project intends to use [conventionnal commit messages](https://conventionalcommits.org/) and the [gitflow](https://nvie.com/posts/a-successful-git-branching-model/) workflow.

### Structure

Scaphandre is a not only a tool, but a framework. Modules dedicated to collect energy comsumption data from the host are called [**Sensors**](docs/sensors).
Modules that are dedicated to send those data to a given channel or remote system are called [**Exporters**](docs/exporters). New Sensors and Exporters are going to be created and all contributions are welcome.

### Roadmap

The ongoing roadmap can be seen [here](https://github.com/hubblo-org/scaphandre/projects/1). Any feature request are welcome, please join us.

### Footprint

In opposition to its name, scaphandre aims to be as light and clean as possible. One of the main focus of the project is to come as close as possible to a 0 overhead, both about resources consumption and power consumption.

### Documentation

Code documentation is [here](https://docs.rs/scaphandre).

Users documentation is [here](docs).