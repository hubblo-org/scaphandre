# Warp10 exporter

![warp10 exporter](images/warp10.png)

## Usage

You can launch the Warp10 exporter this way (running the default powercap_rapl sensor):

	scaphandre warp10

As always exporter's options can be displayed with `-h`:
```
scaphandre-warp10 
Warp10 exporter sends data to a Warp10 host, through HTTP

USAGE:
    scaphandre warp10 [FLAGS] [OPTIONS] --step <step> --write-token <write-token>

FLAGS:
    -h, --help       Prints help information
    -q, --qemu       Time step between measurements, in seconds.
    -V, --version    Prints version information

OPTIONS:
    -H, --host <host>                  Warp10 host's FQDN or IP address to send data to [default: localhost]
    -p, --port <port>                  TCP port to join Warp10 on the host [default: 8080]
    -s, --scheme <scheme>              Either 'http' or 'https' [default: https]
    -S, --step <step>                  Time step between measurements, in seconds. [default: 60]
    -t, --write-token <write-token>    Auth. token to write on Warp10
```
With default options values, the metrics are sent to http://localhost:8080 every 60 seconds

Use -q or --qemu option if you are running scaphandre on a hypervisor. In that case a label with the vm name will be added to all `qemu-system*` processes.
This will allow to easily create charts consumption for each vm and defined which one is the top contributor.

## Metrics exposed

Typically the Warp10 exporter is working in the same way as the riemann exporter regarding metrics. Please look at details in [Prometheus exporter](exporter-prometheus.md) documentations to get the extensive list of metrics available.