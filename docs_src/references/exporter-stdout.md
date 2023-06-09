# Stdout exporter

## Usage

You can launch the stdout exporter this way (running the default powercap_rapl sensor):

	scaphandre stdout

Default behavior is to measure and show metrics periodically during 10 seconds. You can change that timeout with `-t`.
A value of `-t 0` will display top consumers infinitely and must be interrupted with ctrl-c.

Here is how to display metrics during one minute:

    scaphandre stdout -t 60

You can change as well the step measure duration with `-s`. Here is how to display metrics during one minutes with a 5s step:

    scaphandre stdout -t 60 -s 5

You can change the number of top consumers displayed with `-p`. Here is how to display the first 20 top consumers:

    scaphandre stdout -p 20

You can filter the processes to display with `-r`. A warning will be risen if this option is used with `-p` at the same time.
In such case, `-p` behavior is disabled.

The `-r` expected parameter is a regular expression. Details can be found [here](https://docs.rs/regex/1.4.5/regex/#syntax) and tested [here](https://rustexp.lpil.uk/).

Here is how to display power data for the 'scaphandre' process:

    scaphandre stdout -r 'scaphandre'

Metrics provided Scaphandre are documented [here](references/metrics.md). 

Since 1.0.0 the flag `--raw-metrics` displays all metrics available for the host, as a parseable list. This might be useful to list metrics that you would like to fetch afterwards in your monitoring dashboard. Without this flag enabled, Stdout exporter has it's own format and might not show you all available metrics.

As always exporter's options can be displayed with `-h`:

	Write the metrics to the terminal

    Usage: scaphandre stdout [OPTIONS]

    Options:
      -t, --timeout <TIMEOUT>            Maximum time spent measuring, in seconds. If negative, runs forever [default: 10]
      -s, --step <SECONDS>               Interval between two measurements, in seconds [default: 2]
      -p, --processes <PROCESSES>        Maximum number of processes to display [default: 5]
      -r, --regex-filter <REGEX_FILTER>  Filter processes based on regular expressions (example: 'scaph\\w\\w.e')
          --containers                   Monitor and apply labels for processes running as containers
      -q, --qemu                         Apply labels to metrics of processes looking like a Qemu/KVM virtual machine
          --raw-metrics                  Display metrics with their names
      -h, --help                         Print help