# Installation & compilation

## Compile scaphandre from source

We recommand using this version of the rust toolchain or later:

    cargo --version
    cargo 1.48.0 (65cbdd2dc 2020-10-14)
    rustc --version
    rustc 1.48.0 (7eac88abb 2020-11-16)

To be sure to be up to date, you may install rust from the [official website](https://www.rust-lang.org/) instead of your package manager.

To hack *scaph*, or simply be up to date with latest developments, you can download scaphandre from the main branch:

    git clone https://github.com/hubblo-org/scaphandre.git
    cd scaphandre
    cargo build # binary path is target/debug/scaphandre

To use the latest code for a true use case, build for release instead of debug:

    cargo build --release

Binary path is `target/release/scaphandre`.

Depending on your kernel version, you could need to modprobe the module intel_rapl or intel_rapl_common before running scaphandre:

    modprobe intel_rapl_common # or intel_rapl for kernels < 5

## Installation for standard usage

Here are some other ways to install scaphandre depending on your context:

- [run scaphandre in a docker container](quickstart.md)
- [run scaphandre on kubernetes](kubernetes.md)

Brave contributors work on system packages, please have a try and/or contribute to:

- [Debian package](https://github.com/barnumbirr/scaphandre-debian), maintainer: @barnumbirr
- [NixOS package](https://github.com/mmai/scaphandre-flake), maintainer: @mmai

Other tutorials should come, as:

- install scaphandre as a proper systemd service
- scaphandre in your favorite GNU/Linux distribution (need help from packaging gurus !)
- scaphandre on MacOSX
- and more...
