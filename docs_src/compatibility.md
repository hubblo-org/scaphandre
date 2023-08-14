# Compatibility

Scaphandre intends to provide multiple ways to gather power consumption metrics and make understanding tech services footprint possible in many situations. Depending on how you use scaphandre, you may have some restrictions.

To summarize, scaphandre should provide two ways to estimate the power consumption of a service, process or machine. Either by **measuring it**, using software interfaces that give access to hardware metrics, or by **estimating it** if measuring is not an option (this is a [planned feature](https://github.com/hubblo-org/scaphandre/issues/25), not yet implemented as those lines are written, in december 2020).

In scaphandre, the code responsible to collect the power consumption data before any further processing is grouped in components called **sensors**. If you want more details about scaphandre structure, [here are the explanations](explanations/internal-structure.md).

On GNU/Linux [PowercapRAPL sensor](references/sensor-powercap_rapl.md) enables you to measure the power consumption, but it doesn't work in all contexts.

On Windows, [the MsrRAPL sensor](references/sensor-msr_rapl.md), coupled with the [driver responsible to read RAPL MSR's](https://github.com/hubblo-org/windows-rapl-driver/) enables you to do (almost) the same.

| Sensor         | Intel x86 bare metal | AMD x86 bare metal | ARM bare metal | Virtual Machine | Public cloud instance | Container |
| :------------- | :------------------: | :----------------: | :------------: | :-------------: | :-------------------: | :-------: |
| PowercapRAPL (GNU/Linux only)   | [Yes](references/sensor-powercap_rapl.md) | Yes ⚠️  kernel > 5.11 required | We don't know yet | Yes, if on a qemu/KVM hypervisor that runs scaphandre and the [Qemu exporter](references/exporter-qemu.md) | No, until your cloud provider uses scaphandre on its hypervisors | [Depends on what you want](explanations/about-containers.md) |
| MsrRAPL (Windows only)      | Yes               | Probable yes (not tested yet, if you have windows operated AMD gear, please consider [contributing](contributing.md) | No    | Not yet, depends on improvements on the MsrRAPL sensors and overall windows/hypervisors support in Scaphandre |  No, until your cloud provider uses scaphandre on its hypervisors | Might work, not tested yet. If you want to join us in this journey, please consider [contributing](contributing.md) |
| Future estimation based sensor | Future Yes | Future Yes | Future Yes | Future Yes | Future Yes | Future Yes

## Checking RAPL is available on your CPU

Sensors including "RAPL" in their name rely on [RAPL](explanations/rapl-domains.md).

The `pts` and `pln` feature flags ("Intel Package Thermal Status" and "Intel Power Limit Notification" respectively) seem to indicate that RAPL is supported on a CPU. On GNU/Linux, you could be sure of their presence, if this command succeds and matches :

```
egrep "(pts|pln)" /proc/cpuinfo
```