# Prometheus exporter

<img src="https://github.com/hubblo-org/scaphandre/raw/main/docs_src/screen-prometheus.cleaned.png">

## Usage

You can launch the prometheus exporter this way (running the default powercap_rapl sensor):

	scaphandre prometheus

As always exporter's options can be displayed with `-h`:
```
	scaphandre prometheus -h
	scaphandre-prometheus
	Prometheus exporter exposes power consumption metrics on an http endpoint (/metrics is default) in prometheus accepted
	format

	USAGE:
		scaphandre prometheus [FLAGS] [OPTIONS]

	FLAGS:
        --containers    Monitor and apply labels for processes running as containers
		-h, --help       Prints help information
		-q, --qemu       Instruct that scaphandre is running on an hypervisor
		-V, --version    Prints version information

	OPTIONS:
		-a, --address <address>    ipv6 or ipv4 address to expose the service to [default: ::]
		-p, --port <port>          TCP port number to expose the service [default: 8080]
		-s, --suffix <suffix>      url suffix to access metrics [default: metrics]
```
With default options values, the metrics are exposed on http://localhost:8080/metrics.

Use -q or --qemu option if you are running scaphandre on a hypervisor. In that case a label with the vm name will be added to all `qemu-system*` processes.
This will allow to easily create charts consumption for each vm and defined which one is the top contributor.

Metrics provided Scaphandre are documented [here](references/metrics.md).