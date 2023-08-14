# Explanation on RAPL / Running Average Power Limit domains: what we (think we) know so far

RAPL stands for "Running Average Power Limit", it is a feature on Intel/AMD x86 CPU's (manufactured after 2012) that allows to set limits on power used by the CPU and other components. This feature also allows to just get "measurements" (mind the double quotes, as at least part of the numbers RAPL gives are coming from estimations/modeling) of components power usage.

![RAPL domains](rapl.png)

It is composed of "domains", that, in 2023, may include:
- **Core/PP0**: Energy consumed by the CPU Cores themselves.
- **Uncore/PP1**: Energy consumed by components close to the CPU : most of the time it means the embedded GPU chipset. 
- **Dram**: Energy consumed by the memory/RAM sticks
- **Package/PKG**: Includes "Core" and "Uncore". In some documentations and in some of our experiments it seem to include "Dram", but this doesn't seem true in every cases.
- **PSys**: We don't have a clear understanding on this one (yet). But most documentations refer to it with words similar to "PSys: (introduced with Intel Skylake) monitors and controls the thermal and power specifications of the entire SoC and it is useful especially when the source of the power consumption is neither the CPU nor the GPU. For multi-socket server systems, each socket reports its own RAPL values.". To summarize, Psys seems like an interesting metric to get energy consumed by a motherboard and connected components (which includes RAPL usual suspects but also WiFi/Bluetooth cards and probably more). If you want to know more about this metric, we gathered references/sources [here](https://github.com/bpetit/awesome-energy/tree/master#rapl-psys-domain). If you want to help us understanding and documenting better this metric, please consider constributing to the [Energizta project](https://github.com/Boavizta/Energizta/).

RAPL documentation from Intel doesn't necessarily give very precise informations about how RAPL behaves depending on the platform, or about what is included in the calculation. Actively looking for other experimentations/feedbacks/documentations is needed. You might find some informations gathered here: [awesome-energy](https://github.com/bpetit/awesome-energy#rapl). If you have more or more precise informations and are willing to contribute, don't hesitate to open a PR to dev branch on [scaphandre's repository](https://github.com/hubblo-org/scaphandre/tree/dev) (targeting [docs_src folder](https://github.com/hubblo-org/scaphandre/tree/dev/docs_src)) and/or the [awesome-energy](https://github.com/bpetit/awesome-energy) repository.

If you want to know if RAPL is supported by your CPU, please have a look to the end of the [Compatibility](../compatibility.md/) section.