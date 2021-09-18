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

Note

As always exporter's options can be displayed with `-h`:

	$ scaphandre stdout -h
    scaphandre-stdout
    Stdout exporter allows you to output the power consumption data in the terminal

    USAGE:
        scaphandre stdout [OPTIONS]

    FLAGS:
        -h, --help       Prints help information
        -V, --version    Prints version information

    OPTIONS:
        -p, --process <process_number>    Number of processes to display. [default: 5]
        -r, --regex <regex_filter>        Filter processes based on regular expressions (e.g: 'scaph\w\wd.e'). This option
                                          disable '-p' or '--process' one.
        -s, --step <step_duration>        Set measurement step duration in seconds. [default: 2]
        -t, --timeout <timeout>           Maximum time spent measuring, in seconds. 0 means continuous measurement.
                                          [default: 10]

