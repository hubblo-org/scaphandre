# scaphandre

Generic sensor and transmission agent for energy consumption related metrics.

## Getting started

Here are some examples.

Collect energy consumption data, using the [powercap_rapl]() [sensor]() and show the data on the terminal using the [stdout]() [exporter]():

    scaphandre stdout

Collect energy consumption data, using the [powercap_rapl]() [sensor]() and expose data through a prometheus exporter:

    scaphandre prometheus

The complete command is:

    scaphandre -s powercap_rapl stdout # powercap_rapl sensor is the default one

You can also add a different timeout that the default 5 seconds:

    scaphandre stdout -t 10 # measure and print data for 10 seconds

General usage is:

    scaphandre [-s SENSOR] EXPORTER [-t timeout]

Available exporters, as of today, are:

- [stdout](): displays metrics on the standard output/on your terminal
- [prometheus](): exposes metrics as an http endpoint, respecting the [prometheus](https://prometheus.io/) metrics standard

Available sensors, as of today, are:

- [powercap_rapl]()

## Structure

Scaphandre is a not only a tool, but a framework. Modules dedicated to collect energy comsumption data from one or multiple hosts are called *Sensors*.
Modules that are dedicated to send those data to a given channel or remote system are called *Exporters*. New Sensors and Exporters are going to be created and all contributions are welcome.

## Contributing

Feel free to propose pull requests, or open new issues at will. Scaphandre is a collaborative project and all opinions and propositions shall be heard and studied. The contributions will be received with kindness, gratitude and with an open mind. Remember that we are all dwarfs standing on the shoulders of giants. We all have to learn from others and to give back, with due mutual respect.

This project intends to use [conventionnal commit messages](conventionalcommits.org/).