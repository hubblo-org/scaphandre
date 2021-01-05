# Troubleshooting

### I get a **permission denied** error when I run scaphandre, no matter what is the exporter

On some Linux distributions (ubuntu 20.04 for sure), the energy counters files that the [PowercapRAPL sensor](references/sensor-powercap_rapl.md) uses, are owned by root. (since late 2020)

To ensure this is your issue and fix that quickly you can run the [init.sh](https://raw.githubusercontent.com/hubblo-org/scaphandre/main/init.sh) script:

    bash init.sh

Then run scaphandre. If it does not work, the issue is somewhere else.

### I get a **no such device** error, the intel_rapl of intel_rapl_common kernel modules are present

It can mean that your cpu doesn't support RAPL. Please refer to the [compatibility](compatibility.md) section to be sure.