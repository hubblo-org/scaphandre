# How scaphandre computes per process power consumption

As you can see with the [prometheus exporter reference](../references/exporter-prometheus.md), scaphandre exporters can provide process level power consumption metrics. This section will explain how it is done and how it may be improved in the future.

## Some details about RAPL

We'll talk here about the case where scaphandre is able to effectively measure the power consumption of the host (see [compatibility](../compatibility.md) section for more on sensors and their prerequesites) and specifically about the [PowercapRAPL](../references/sensor-powercap_rapl.md) sensor.

Let's clarify what's happening when you collect metrics with scaphandre and this sensor.
RAPL stands for [Running Average Power Limit](https://01.org/blogs/2014/running-average-power-limit-%E2%80%93-rapl). It's a technnology embedded in most Intel and AMD x86 CPUs produced afeter 2012. Thanks to this technology it is possible to get the total energy consumption of the CPU, of the consumption per CPU socket, plus in some cases, the consumption of the DRAM controller. In most cases it represents the vast majority of the energy consumption of the machine (except when running GPU intensive workloads, for example). Further improvements shall be made in scaphandre to fully measure the consumption when GPU are involved (or a lot of hard drives on the same host...).

Between scaphandre and those data is the powercap kernel module that writes the energy consumption in files. Scaphandre, reads those files, stores the data in buffer and then allows for more processing through the exporters.

## How to get the consumption of one process ?

The PowercapRAPL sensor does actually some more than just collecting those energy consumption metrics (and casting it in power consumption metrics). Every time the exporter asks for a measurement (either periodically like in the [Stdout](../references/exporter-stdout.md) exporter, or every time a request comes like for the Prometheus exporter) the sensor reads the values of the energy counters from powercap, stores those values and does the same for the CPU usage statistics of the CPU (the one you can see in `/proc/stats`) and for each running process on the machine at that time (see `/proc/PID/stats`). With those data it is possible to compute the ratio of CPU time actively spent for a given PID on the CPU time actively spent doing something. With this ratio we can then get the subset of power consumption that is related to that PID on a given timeframe (between two measurement requests).

## How to get the consumption of an application/a service ?

Services and programs are often not running only one PID. It's needed to aggregate the consumption of all related PIDs to know what this service is actually consuming. 

To do that, in the current state of scaphandre development, you can use the Prometheus exporter, and then use Prometheus TSDB and query language capabilities. You'll find examples looking at the graphs and queries [here](https://metrics.hubblo.org). In a near future, more advanced features may be implemented in scaphandre to allow such classification even if you don't have access to a proper TSDB.