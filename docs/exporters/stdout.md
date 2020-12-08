# Stdout exporter

## Usage

You can launch the stdout exporter this way (running the default powercap_rapl sensor):

	scaphandre stdout

Default behavior is to measure and show metrics periodically during 10 seconds. You can change that timeout with `-t`. Here is how to display metrics during one minute:

    scaphandre stdout -t 60

As always exporter's options can be displayed with `-h`:

	$ scaphandre stdout -h
    scaphandre-stdout 
    Stdout exporter allows you to output the power consumption data in the terminal.

    USAGE:
        scaphandre stdout [OPTIONS]

    FLAGS:
        -h, --help       Prints help information
        -V, --version    Prints version information

    OPTIONS:
        -t, --timeout <timeout>    Maximum time spent measuring, in seconds. [default: 10]