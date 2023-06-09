# Warp10 exporter

![warp10 exporter](images/warp10.png)

## Usage

You can launch the Warp10 exporter this way (running the default powercap_rapl sensor):

	scaphandre warp10

You need a token to be able to push data to a [warp10](https://warp10.io) instance.
The `SCAPH_WARP10_WRITE_TOKEN` env var can be used to make it available to scaphandre.
Please refer to the warp10 documentation to know how to get the token in the first place.

As always exporter's options can be displayed with `-h`:
```
scaphandre-warp10 
Warp10 exporter sends data to a Warp10 host, through HTTP

USAGE:
    scaphandre warp10 [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -q, --qemu       Tells scaphandre it is running on a Qemu hypervisor.
    -V, --version    Prints version information

OPTIONS:
    -H, --host <host>                  Warp10 host's FQDN or IP address to send data to [default: localhost]
    -p, --port <port>                  TCP port to join Warp10 on the host [default: 8080]
    -s, --scheme <scheme>              Either 'http' or 'https' [default: http]
    -S, --step <step>                  Time step between measurements, in seconds. [default: 30]
    -t, --write-token <write-token>    Auth. token to write on Warp10
```
With default options values, the metrics are sent to http://localhost:8080 every 60 seconds

Use -q or --qemu option if you are running scaphandre on a hypervisor. In that case a label with the vm name will be added to all `qemu-system*` processes.
This will allow to easily create charts consumption for each vm and defined which one is the top contributor.

Metrics provided Scaphandre are documented [here](references/metrics.md). 