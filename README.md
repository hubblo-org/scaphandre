# scaphandre

Generic sensor and transmission agent for energy consumption related metrics.

## Usage

Example: use scaphandre to collect energy consumption data, using the [powercap_rapl sensor]() and the [stdout exporter]() on the local host.

    scaphandre -s powercap_rapl -s stdout

## Structure

Scaphandre is a not only a tool, but a framework. Modules dedicated to collect energy comsumption data from one or multiple hosts are called *Sensors*.
Modules that are dedicated to send those data to a given channel or remote system are called *Exporters*. New Sensors and Exporters are going to be created and all contributions are welcome.

## Contributing

Feel free to propose pull requests, or open new issues at will. Scaphandre is a collaborative project and all opinions and propositions shall be heard and studied. The contributions will be received with kindness, gratitude and with an open mind. Remember that we are all dwarfs standing on the shoulders of giants. We all have to learn from others and to give back, with due mutual respect.

This project intends to use [conventionnal commit messages](conventionalcommits.org/).