# Quickstart

Here you'll see how to deploy and benefit from Scaphandre quickly.

The quickest way to get scaphandre is to download the [latest binary](https://github.com/hubblo-org/releases) suitable for your environment.

Uncompress the file:

    gunzip scaphandre-v0.1.1-Ubuntu_20.04-x86_64.gz && mv scaphandre-v0.1.1-Ubuntu_20.04-x86_64 scaphandre

You can now run scaphandre. Let's see if you can get metrics directly in your terminal:

    ./scaphandre stdout -t 15

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

At that point, you're ready to use scaphandre the way you like. The Stdout exporter is very basic and other exporters should allow you to use and send those metrics the way you like.

The [prometheus exporter](references/exporter-prometheus.md), for example, allows you to expose power consumption metrics as an HTTP endpoint that can be scrapped by a [prometheus](https://prometheus.io) instance:

    scaphandre prometheus

To validate that the metrics are available, send an http request from another terminal:

    curl -s http://localhost:8080/metrics

[Here](https://metrics.hubblo.org) you can see examples of graphs you can get thanks to scaphandre, the prometheus exporter, prometheus and [grafana](https://grafana.com/).