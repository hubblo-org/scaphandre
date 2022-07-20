# Compatibility

Scaphandre intends to provide multiple ways to gather power consumption metrics and make understanding tech services footprint possible in many situations. Depending on how you use scaphandre, you may have some restrictions.

To summarize, scaphandre should provide two ways to estimate the power consumption of a service, process or machine. Either by **measuring it**, using software interfaces that give access to hardware metrics, or by **estimating it** if measuring is not an option (this is a [planned feature](https://github.com/hubblo-org/scaphandre/issues/25), not yet implemented as those lines are written, in december 2020).

In scaphandre, the code responsible to collect the power consumption data before any further processing is grouped in components called **sensors**. If you want more details about scaphandre structure, [here are the explanations](explanations/internal-structure.md).

The [PowercapRAPL sensor](references/sensor-powercap_rapl.md) enables you to measure the power consumption, it is the most precise solution, but it doesn't work in all contexts. A future sensor is to be developed to support other use cases. Here is the current state of scaphandre's compatibility:

| Sensor         | Intel x86 bare metal | AMD x86 bare metal | ARM bare metal | Virtual Machine | Public cloud instance | Container |
| :------------- | :------------------: | :----------------: | :------------: | :-------------: | :-------------------: | :-------: |
| PowercapRAPL   | [Yes](references/sensor-powercap_rapl.md) | Yes ⚠️  kernel > 5.11 required | We don't know yet | Yes, if on a qemu/KVM hypervisor that runs scaphandre and the [Qemu exporter](references/exporter-qemu.md) | No, until your cloud provider uses scaphandre on its hypervisors | [Depends on what you want](explanations/about-containers.md) |
| Future estimation based sensor | Future Yes | Future Yes | Future Yes | Future Yes | Future Yes |

| Sensor        | GNU/Linux        | Windows                                | MacOS |
| :-----------: | :--------------: | :------------------------------------: | :---: |
| PowercapRAPL  | Yes (see above)  | No                                     | No    |
| MsrRAPL       | No               | Yes (tested on windows 10/server 2019) | No    |