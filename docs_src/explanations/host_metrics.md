# Explanations about host level power and energy metrics.

This is true starting **from Scaphandre >= 1.0.**

There are several [metrics](../references/metrics.md) available at the host level in Scaphandre:
- `scaph_host_power_microwatts` : always returned, computed from Record structs made from `scaph_host_energy_microjoules` metric
- `scaph_host_energy_microjoules` : always returned, either one value or a sum of values coming directly from RAPL counters (`energy_uj` files or direct read from an MSR)
- `scaph_host_rapl_psys_microjoules` : is available only when the PSYS [RAPL domain](explanations/rapl-domains.md) is available on the machine.

In addition to those metrics, you might want to build, on your time series database, the sum of process_ metrics to have a view of the weight of all processes on the host power. Using Prometheus, it would look like: `sum(scaph_process_power_consumption_microwatts{hostname="$hostname"}) / 1000000`, to get it in Watts.

Let's explain the relationship between those metrics, and what you could expect.

`host_power` metric will return :
1. If PSYS domain is available, a computed power coming from PSYS energy records
2. If not, a computed power which is the sum of per-socket power (PKG RAPL domain) + DRAM RAPL domain power

Briefly explained (see [RAPL domains](explanations/rapl-domains.md) for detailled explanations), PSYS covers most components on the machine ("all components connected to the SoC / motherboard" according to most documentations), so we return this wider ranged metric when available. If not we use a combination of PKG domain, that includes CPU and integrated GPU power, and DRAM domain, that includes memory power. The first options gives higher figures than the second, for now.

Suming the power of all processes, if the machine is mostly IDLE, you'll get a tiny percentage of the host machine, most likely. The difference between host power and the sum of processes power can be accounted as "power due to IDLE activity", in other words the power your machine demands for "doing nothing". The higher this difference on a long period of time (better seen as a graph), the higher chance that there is room for improvement in moving the workloads to another machine and shut the current machine down (and make it available for another project or to another organization to prevent from buying a new machine).

**Warning:** that being said, the way per-process power is computed is still biased and shall be improved in the following versions of Scaphandre. For now, the main key for allocation is CPU time. As host level power metrics include power usage of more and more components on the machine (work in progress) this allocation key will be more and more inaccurate. Future versions of this allocation model should include keys regarding the activity of other components than CPU. Enabling a better set of allocation keys for per-process power is part of the [roadmap](https://github.com/hubblo-org/scaphandre/projects/1).
