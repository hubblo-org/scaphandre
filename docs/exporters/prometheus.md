# Prometheus exporter

<img src="https://github.com/hubblo-org/scaphandre/raw/main/screen-prometheus.cleaned.png">

## Usage

You can launch the prometheus exporter this way (running the default powercap_rapl sensor):

	scaphandre prometheus

As always exporter's options can be displayed with `-h`:

	scaphandre prometheus -h
	scaphandre-prometheus 
	prometheus exporter exposes power consumption metrics on an http endpoint (/metrics is default) in prometheus accepted
	format.

	USAGE:
		scaphandre --sensor <sensor> prometheus [OPTIONS]

	FLAGS:
		-h, --help       Prints help information
		-V, --version    Prints version information

	OPTIONS:
		-a, --address <address>    ipv6 or ipv4 address to expose the service to [default: ::]
		-p, --port <port>          TCP port number to expose the service [default: 8080]
		-s, --suffix <suffix>      url suffix to access metrics [default: metrics]

With default options values, the metrics are exposed on http://localhost:8080/metrics.

## Metrics exposed

All metrics have a HELP section provided on /metrics (or whatever suffix you choosed to expose them).

Here are some key metrics that you will most probably be interested in:

- `scaph_host_power_microwatts`: Power measurement on the whole host, in microwatts (GAUGE)
- `scaph_process_power_consumption_microwatts{exe="$PROCESS_EXE",pid="$PROCESS_PID",cmdline="path/to/exe --and-maybe-options"}`: Power consumption due to the process, measured on at the topology level, in microwatts. PROCESS_EXE being the name of the executable and PROCESS_PID being the pid of the process. (GAUGE)

And some more deep metrics that you may want if you need to make more complex calculations and data processing:

- `scaph_host_energy_microjoules` : Energy measurement for the whole host, as extracted from the sensor, in microjoules. (COUNTER)
- `scaph_host_energy_timestamp_seconds`: Timestamp in seconds when hose_energy_microjoules has been computed. (COUNTER)
- `scaph_socket_power_microwatts{socket_id="$SOCKET_ID"}`: Power measurement relative to a CPU socket, in microwatts. SOCKET_ID being the socket numerical id (GAUGE)