# Installation (GNU/Linux)

Depending on your kernel version, you could need to modprobe the module intel_rapl or intel_rapl_common before running scaphandre:

    modprobe intel_rapl_common # or intel_rapl for kernels < 5

## Docker
To quickly run scaphandre in your terminal you may use [docker](https://www.docker.com/):

    docker run -v /sys/class/powercap:/sys/class/powercap -v /proc:/proc -ti hubblo/scaphandre stdout -t 15

## Debian/Ubuntu
On Debian or Ubuntu, you can use the available `.deb` [package](https://github.com/barnumbirr/scaphandre-debian).

    VERSION="1.0.0-1" ARCH="amd64" DIST="bookworm" && \
    wget https://github.com/barnumbirr/scaphandre-debian/releases/download/v$VERSION/scaphandre_$VERSION\_$ARCH\_$DIST.deb && \
    dpkg -i scaphandre_$VERSION\_$ARCH\_$DIST.deb && \
    rm scaphandre_$VERSION\_$ARCH\_$DIST.deb

## Run the binary
Once you downloaded or built a binary, you'd run:

    scaphandre stdout -t 15

Here are some other ways to install scaphandre depending on your context:

- [quickly try the project with docker-compose/docker stack](docker-compose.md)
- [run scaphandre on kubernetes](kubernetes.md)
- [run scaphandre on RHEL, with prometheus-push mode](../how-to_guides/install-prometheuspush-only-rhel.md)

Kudos to contributors who work on system packages, please have a try and/or contribute to:

- [Debian package](https://github.com/barnumbirr/scaphandre-debian), maintainer: @barnumbirr
- [NixOS package](https://github.com/mmai/scaphandre-flake), maintainer: @mmai

Other tutorials should come, as:

- install scaphandre as a proper systemd service
- scaphandre in your favorite GNU/Linux distribution (need help from packaging gurus !)
- scaphandre on MacOSX
- and more...
