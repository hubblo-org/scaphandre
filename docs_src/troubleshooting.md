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

On ubuntu 20.01 and 20.10, try to install `linux-modules-extra-$(uname -r)` with apt. Then you should be able to `modprobe intel_rapl_common`.

### On an AMD cpu machine, I get the following stracktrace

    scaphandre::sensors::powercap_rapl: Couldn't find intel_rapl modules.
    thread 'main' panicked at 'Trick: if you are running on a vm, do not forget to use --vm parameter invoking scaphandre at the command line', src/sensors/mod.rs:238:18
    stack backtrace:
       0: rust_begin_unwind
                 at /build/rust/src/rustc-1.49.0-src/library/std/src/panicking.rs:495:5
       1: core::panicking::panic_fmt
                 at /build/rust/src/rustc-1.49.0-src/library/core/src/panicking.rs:92:14
       2: core::option::expect_failed
                 at /build/rust/src/rustc-1.49.0-src/library/core/src/option.rs:1260:5
       3: core::option::Option<T>::expect
                 at /build/rust/src/rustc-1.49.0-src/library/core/src/option.rs:349:21
       4: scaphandre::sensors::Topology::add_cpu_cores
                 at ./src/sensors/mod.rs:234:26
       5: <scaphandre::sensors::powercap_rapl::PowercapRAPLSensor as scaphandre::sensors::Sensor>::generate_topology
                 at ./src/sensors/powercap_rapl.rs:106:9
       6: <scaphandre::sensors::powercap_rapl::PowercapRAPLSensor as scaphandre::sensors::Sensor>::get_topology
                 at ./src/sensors/powercap_rapl.rs:112:24
       7: scaphandre::exporters::stdout::StdoutExporter::new
                 at ./src/exporters/stdout.rs:51:30
       8: scaphandre::run
                 at ./src/lib.rs:60:28
       9: scaphandre::main
                 at ./src/main.rs:91:5
      10: core::ops::function::FnOnce::call_once
                 at /build/rust/src/rustc-1.49.0-src/library/core/src/ops/function.rs:227:5
    note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.

We verified that scaphandre (and especially the powercap_rapl sensor) works on AMD Zen processors with a Linux kernel **5.11 or later**. Before that kernel version, it won't probably work as the [drivers](https://www.phoronix.com/scan.php?page=news_item&px=AMD-Zen-PowerCap-RAPL-5.11) needed to feed powercap with rapl data are not present.

### Trying to build the project I get this error

    error: linker `cc` not found
      |
      = note: No such file or directory (os error 2)

    error: aborting due to previous error

    error: could not compile `log`

You need compiling tooling. On Ubuntu/Debian, run:

     sudo apt install build-essential
