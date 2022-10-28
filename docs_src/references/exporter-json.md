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

As always exporter's options can be displayed with `-h`:

	$ scaphandre json -h
    JSON exporter allows you to output the power consumption data in a json file

    USAGE:
        scaphandre json [FLAGS] [OPTIONS]

    FLAGS:
            --containers    Monitor and apply labels for processes running as containers
        -h, --help          Prints help information
        -V, --version       Prints version information

    OPTIONS:
        -f, --file <file_path>                         Destination file for the report. [default: ]
        -m, --max-top-consumers <max_top_consumers>    Maximum number of processes to watch. [default: 10]
        -s, --step <step_duration>                     Set measurement step duration in second. [default: 2]
        -n, --step_nano <step_duration_nano>           Set measurement step duration in nano second. [default: 0]
        -t, --timeout <timeout>                        Maximum time spent measuring, in seconds.
