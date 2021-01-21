# Troubleshooting

### I get a **permission denied** error when I run scaphandre, no matter what is the exporter

On some Linux distributions (ubuntu 20.04 for sure), the energy counters files that the [PowercapRAPL sensor](references/sensor-powercap_rapl.md) uses, are owned by root. (since late 2020)

To ensure this is your issue and fix that quickly you can run the [init.sh](https://raw.githubusercontent.com/hubblo-org/scaphandre/main/init.sh) script:

    bash init.sh

Then run scaphandre. If it does not work, the issue is somewhere else.

### I get a **no such device** error, the intel_rapl of intel_rapl_common kernel modules are present

It can mean that your cpu doesn't support RAPL. Please refer to the [compatibility](compatibility.md) section to be sure.

### I can't mount the required kernel modules, getting a `Could'nt find XXX modules` error

If you are in a situation comparable to [this one](https://github.com/hubblo-org/scaphandre/issues/59), you may need to install additional packages.

On ubuntu 20.01 and 20.10, try to install `linux-modules-extra-$(uname-r)` with apt. Then you should be able to `modprobe intel_rapl_common`.
