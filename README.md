# scaphandre

<img src="https://github.com/hubblo-org/scaphandre/raw/main/scaphandre.cleaned.png" width="200">

---

Generic sensor and transmission agent for energy consumption related metrics.

## Getting started

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

    scaphandre [-s SENSOR] EXPORTER [-t timeout]

Available exporters are:

- [stdout](docs/exporters/stdout.md): displays metrics on the standard output/on your terminal
- [prometheus](docs/exporters/prometheus.md): exposes metrics as an http endpoint, respecting the [prometheus](https://prometheus.io/) metrics standard

Available sensors, as of today, are:

- [powercap_rapl](docs/sensors/powercap_rapl.md)

## Structure

Scaphandre is a not only a tool, but a framework. Modules dedicated to collect energy comsumption data from one or multiple hosts are called *Sensors*.
Modules that are dedicated to send those data to a given channel or remote system are called *Exporters*. New Sensors and Exporters are going to be created and all contributions are welcome.

## Contributing

Feel free to propose pull requests, or open new issues at will. Scaphandre is a collaborative project and all opinions and propositions shall be heard and studied. The contributions will be received with kindness, gratitude and with an open mind. Remember that we are all dwarfs standing on the shoulders of giants. We all have to learn from others and to give back, with due mutual respect.

This project intends to use [conventionnal commit messages](conventionalcommits.org/).