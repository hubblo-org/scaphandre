# Installation (GNU/Linux)

Depending on your kernel version, you could need to modprobe the module intel_rapl or intel_rapl_common before running scaphandre:

    modprobe intel_rapl_common # or intel_rapl for kernels < 5

To quickly run scaphandre in your terminal you may use [docker](https://www.docker.com/):

    docker run -v /sys/class/powercap:/sys/class/powercap -v /proc:/proc -ti hubblo/scaphandre stdout -t 15

Or if you downloaded or built a binary, you'd run:

    scaphandre stdout -t 15

Here are some other ways to install scaphandre depending on your context:

- [quickly try the project with docker-compose/docker stack](docker-compose.md)
- [run scaphandre on kubernetes](kubernetes.md)

Brave contributors work on system packages, please have a try and/or contribute to:

- [Debian package](https://github.com/barnumbirr/scaphandre-debian), maintainer: @barnumbirr
- [NixOS package](https://github.com/mmai/scaphandre-flake), maintainer: @mmai

Other tutorials should come, as:

- install scaphandre as a proper systemd service
- scaphandre in your favorite GNU/Linux distribution (need help from packaging gurus !)
- scaphandre on MacOSX
- and more...
