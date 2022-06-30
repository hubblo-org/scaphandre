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

### Trying to build the project I get "linker `cc` not found"

    error: linker `cc` not found
      |
      = note: No such file or directory (os error 2)

    error: aborting due to previous error

    error: could not compile `log`

You need compiling tooling. On Ubuntu/Debian, run:

     sudo apt install build-essential

## Trying to build the project I get "pkg_config fail: Failed to run ... openssl"

Full error may look like that :

    run pkg_config fail: "Failed to run `\"pkg-config\" \"--libs\" \"--cflags\" \"openssl\"`: No such file or directory (os error 2)"

    --- stderr
    thread 'main' panicked at '

    Could not find directory of OpenSSL installation, and this `-sys` crate cannot
    proceed without this knowledge. If OpenSSL is installed and this crate had
    trouble finding it,  you can set the `OPENSSL_DIR` environment variable for the
    compilation process.

    Make sure you also have the development packages of openssl installed.
    For example, `libssl-dev` on Ubuntu or `openssl-devel` on Fedora.

    If you're in a situation where you think the directory *should* be found
    automatically, please open a bug at https://github.com/sfackler/rust-openssl
    and include information about your system as well as this message.

    $HOST = x86_64-unknown-linux-gnu
    $TARGET = x86_64-unknown-linux-gnu
    openssl-sys = 0.9.66


    It looks like you're compiling on Linux and also targeting Linux. Currently this
    requires the `pkg-config` utility to find OpenSSL but unfortunately `pkg-config`
    could not be found. If you have OpenSSL installed you can likely fix this by
    installing `pkg-config`.

    ', /home/bpetit/.cargo/registry/src/github.com-1ecc6299db9ec823/openssl-sys-0.9.66/build/find_normal.rs:174:5
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

On Debian/Ubuntum the solution would be to install both pkg-config and libssl-dev :

    apt install pkg-config libssl-dev
