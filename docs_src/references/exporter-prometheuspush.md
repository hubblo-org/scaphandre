# PrometheusPush Exporter for Prometheus Push Gateway

## Usage

You can launch the prometheus exporter this way:

	scaphandre prometheus-push

As always exporter's options can be displayed with `-h`:
```
	scaphandre prometheus-push -h
	Push metrics to Prometheus Push Gateway

	Usage: scaphandre prometheus-push [OPTIONS]

	Options:
	  -H, --host <HOST>      IP address (v4 or v6) of the metrics endpoint for Prometheus [default: localhost]
	  -p, --port <PORT>      TCP port of the metrics endpoint for Prometheus [default: 9091]
	      --suffix <SUFFIX>  [default: metrics]
	  -S, --scheme <SCHEME>  [default: http]
	  -s, --step <STEP>      [default: 5]
	      --qemu             Apply labels to metrics of processes that look like a Qemu/KVM virtual machine
	      --containers       Apply labels to metrics of processes running as containers
	  -j, --job <JOB>        Job name to apply as a label for pushed metrics [default: scaphandre]
	      --no-tls-check     Don't verify remote TLS certificate (works with --scheme="https")
	  -h, --help             Print help
```
With default options values, the metrics are sent to http://localhost:9091/metrics

## Metrics exposed

Metrics exposed are the same as the Prometheus (pull mode) exporter.

Push gateway's grouping key for each host is in the form `job/scaphandre/instance/${HOSTNAME}` with HOSTNAME being the hostname of the host sending metrics.