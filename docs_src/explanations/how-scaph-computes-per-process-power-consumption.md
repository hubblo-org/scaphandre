# T

As you may have seen in the [prometheus exporter reference](../references/exporter-prometheus.md), scaphandre exporters may provide process level power consumption metrics. This section will explain how it is done and how it may be improved in the future.

## Some details about RAPL

Most probably you are using the PowercapRAPL sensor to collect the metrics with scaphandre (this is the only one ready as those lines are written). Let's clarify what's happening.
RAPL stands for Running Average Power Limit. It's a technology embedded in most Intel and AMD x86 CPUs produced afeter 2012. The counters provided a