# Getting started

To install Scaphandre, depending on your platform, see:
+ [Installation on GNU/Linux](installation_linux.md) section.
+ [Installation on Windows](installation_windows.md) section.

If you want to contribute or just play with the code and need to compile Scaphandre, see :
+ [Compilation on GNU/Linux](compilation_linux.md).
+ [Compilation on Windows](compilation_windows.md).

Depending on your kernel version, you could need to modprobe the module intel_rapl or intel_rapl_common first:

    modprobe intel_rapl_common # or intel_rapl for kernels < 5

To quickly run scaphandre in your terminal you may use [docker](https://www.docker.com/):

    docker run -v /sys/class/powercap:/sys/class/powercap -v /proc:/proc -ti hubblo/scaphandre stdout -t 15

Or if you downloaded or built a binary, you'd run:

    scaphandre stdout -t 15

## Running scaphandre on Fedora / CentOS Stream / RHEL (or any distribution using SELinux) with podman

Running scaphandre with podman on a distribution using SELinux may fail because of access denied to `/proc` files.

To make it work you should run scaphandre in privileged mode :

    podman run --privileged ...

You'll find explanation of this requirement here : [#106](https://github.com/hubblo-org/scaphandre/issues/106).

## Output

Here we are using the stdout [exporter](../explanations/internal-structure.md) to print current power consumption usage in the terminal during 15 seconds.

You should get an output like:

    Host:	9.391334 W	Core		Uncore		DRAM
    Socket0	9.392    W	1.497082 W
    Top 5 consumers:
    Power	PID	Exe
    4.808363 W	642	"/usr/sbin/dockerd"
    4.808363 W	703	"/usr/bin/docker-containerd"
    4.808363 W	1028	"/usr/local/bin/redis-server"
    0 W	1	"/usr/lib/systemd/systemd"
    0 W	2	""
    ------------------------------------------------------------

Let's briefly describe what you see here. First Line is the power consumption of the machine (between the two last measurements).
Second line is the power consumption of the first CPU socket plus the detail by RAPL Domain.
If you have more than one CPU Socket, you'll have multiple *SocketX* lines.
Then you have the 5 processes consuming the most power during the last two measurements.

If you don't get this output and get an error, jump to the [Troubleshooting](../troubleshooting.md) section of the documentation.

## Going further

At that point, you're ready to use scaphandre. The Stdout exporter is very basic and other exporters should allow you to use and send those metrics the way you like.

The [prometheus exporter](references/exporter-prometheus.md), for example, allows you to expose power consumption metrics as an HTTP endpoint that can be scrapped by a [prometheus](https://prometheus.io) instance:

    docker run -v /sys/class/powercap:/sys/class/powercap -v /proc:/proc -p 8080:8080 -ti hubblo/scaphandre prometheus

Here is the same command with a simple binary:

    scaphandre prometheus

To validate that the metrics are available, send an http request from another terminal:

    curl -s http://localhost:8080/metrics

[Here](https://metrics.hubblo.org) you can see examples of graphs you can get thanks to scaphandre, the prometheus exporter, prometheus and [grafana](https://grafana.com/).
