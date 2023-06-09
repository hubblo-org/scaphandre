# JSON exporter

## Usage

You can launch the JSON exporter this way (running the default powercap_rapl sensor):

	scaphandre json

Default behavior is to measure and show metrics periodically during 10 seconds. You can change that timeout with `-t`. Here is how to display metrics during one minute:

    scaphandre json -t 60

You can change as well the step measure duration with -s. Here is how to display metrics during one minutes with a 5s step:

    scaphandre json -t 60 -s 5

If you want a faster interval you can use option -n (for nano seconds). Here is how to display metrics during 10s with a 100ms step:

    scaphandre json -t 10 -s 0 -n 100000000

By default, JSON is printed in the terminal, to write result in a file you can provide a path with option -f:

    scaphandre json -t 10 -s 0 -n 100000000 -f report.json

To get informations about processes that are running in containers, add `--containers`:

    scaphandre --no-header json --containers --max-top-consumers=15 | jq

Since 1.0.0 you can filter the processes, either by their process name with `--process-regex`, or by the name of the container they run in with `--container-regex` (needs the flag `--containers` to be active as well).

As always exporter's options can be displayed with `-h`:

	Write the metrics in the JSON format to a file or to stdout

    Usage: scaphandre json [OPTIONS]

    Options:
      -t, --timeout <TIMEOUT>
              Maximum time spent measuring, in seconds. If unspecified, runs forever
      -s, --step <SECONDS>
              Interval between two measurements, in seconds [default: 2]
          --step-nano <NANOSECS>
              Additional step duration in _nano_ seconds. This is added to `step` to get the final duration [default: 0]
          --max-top-consumers <MAX_TOP_CONSUMERS>
              Maximum number of processes to watch [default: 10]
      -f, --file <FILE>
              Destination file for the report (if absent, print the report to stdout)
          --containers
              Monitor and apply labels for processes running as containers
          --process-regex <PROCESS_REGEX>
              Filter processes based on regular expressions (example: 'scaph\\w\\w.e')
          --container-regex <CONTAINER_REGEX>
              Filter containers based on regular expressions
          --resources
              Monitor and incude CPU, RAM and Disk usage per process
      -h, --help
              Print help

Metrics provided Scaphandre are documented [here](references/metrics.md).