# MSR_RAPL sensor

## Pre-requesites

At the time those lines are written, this sensor works only on:

- OS: Windows 10/Windows Server 2016, Windows Server 2019
- Intel and AMD x86 CPUs, produced after 2012 (or some laptop cpu prior to 2012)

This sensor needs the [RAPL MSR-based driver](https://github.com/hubblo-org/windows-rapl-driver/) to be installed.

## Usage

To explicitely call the powercap_rapl sensor from the command line use:

    scaphandre -s msr_rapl EXPORTER # EXPORTER being the exporter name you want to use

You can see arguments available from the cli for this sensors with:

    scaphandre -s msr_rapl -h

Please refer to doc.rs code documentation for more details.

## Options available

TODO

## Environment variables

TODO

## Troubleshooting

TODO
