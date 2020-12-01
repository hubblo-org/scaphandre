# Scaphandre

<img src="https://github.com/hubblo-org/scaphandre/raw/main/scaphandre.cleaned.png" width="200">

---

Scaphandre (or "scaf" for busy people) is a metrology agent dedicated for electrical [power](https://en.wikipedia.org/wiki/Electric_power) consumption related metrics. The goal of the project is to permit to any company or individual to measure the power consumption of its tech services and get those data in a convenient form, sending it though any monitoring or data analysis toolchain.

**Scaphandre** means *heavy* **diving suit** in [:fr:](https://fr.wikipedia.org/wiki/Scaphandre_%C3%A0_casque). It comes from the idea that tech related services often don't track their power consumption and thus don't expose it to their clients. Most of the time the reason is a presumed bad [ROI](https://en.wikipedia.org/wiki/Return_on_investment). Scaphandre makes, for tech providers and tech users, easier and cheaper to go under the surface to bring back the desired power consumption metrics, take better sustainability focussed decisions, and then show the metrics to their clients to allow them to do the same.

In opposition to its name, scaphandre aims to be as light and clean as possible. One of the main focus of the project is to come as close as possible to a 0 overhead, both about resources consumption and power consumption.

See the [why](docs/why.md) section for more about the goals of the project.

![Rust](https://github.com/hubblo-org/scaphandre/workflows/Rust/badge.svg?branch=main)
![https://gitter.im/hubblo-org/scaphandre?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge](https://badges.gitter.im/Join%20Chat.svg)

---

## Getting started

Installation steps are described [here](#Installation).

Here are some examples.

To show power consumption metrics in your terminal, run:

    scaphandre stdout

To expose power consumption metrics as a [prometheus](https://prometheus.io) exporter (as an http endpoint):

    scaphandre prometheus

Metrics are now available on http://localhost:8080/metrics.

A more complete command would be:

    scaphandre -s powercap_rapl stdout

As you can see `-s` option allows you to select the **sensor**, which is the scaphandre component in charge of collecting power consumption metrics.

You can also add a different timeout that the default 5 seconds:

    scaphandre stdout -t 10 # measure and print data for 10 seconds

General usage is:

    scaphandre [-s SENSOR] EXPORTER [OPTIONS]

You can get available options for both

Available exporters are:

- [stdout](docs/exporters/stdout.md): displays metrics on the standard output/on your terminal
- [prometheus](docs/exporters/prometheus.md): exposes metrics as an http endpoint, respecting the [prometheus](https://prometheus.io/) metrics standard

Available sensors, as of today, are:

- [powercap_rapl](docs/sensors/powercap_rapl.md)

## Installation

To hack scaf, or simply be up to date with latest developments, you can download scaphandre from the main branch:

    git clone https://github.com/hubblo-org/scaphandre.git && cd scaphandre
    cargo build
    target/debug/scaphandre prometheus

To use the latest code for a true use case, build for release instead of debug:

    cargo build --release
    target/release/scaphandre prometheus

## Structure

Scaphandre is a not only a tool, but a framework. Modules dedicated to collect energy comsumption data from one or multiple hosts are called *Sensors*.
Modules that are dedicated to send those data to a given channel or remote system are called *Exporters*. New Sensors and Exporters are going to be created and all contributions are welcome.

## Contributing

Feel free to propose pull requests, or open new issues at will. Scaphandre is a collaborative project and all opinions and propositions shall be heard and studied. The contributions will be received with kindness, gratitude and with an open mind. Remember that we are all dwarfs standing on the shoulders of giants. We all have to learn from others and to give back, with due mutual respect.

This project intends to use [conventionnal commit messages](conventionalcommits.org/).