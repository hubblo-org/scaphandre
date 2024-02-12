<p align="center">
    <img src="https://github.com/hubblo-org/scaphandre/raw/main/docs_src/scaphandre.cleaned.png" width="200">
</p>
<h1 align="center">
  Scaphandre
</h1>

<h3 align="center">
    Your tech stack doesn't need so much energy ⚡
</h3>

---

Scaphandre *[skafɑ̃dʁ]* is a metrology agent dedicated to electric [power](https://en.wikipedia.org/wiki/Electric_power) and energy consumption metrics. The goal of the project is to permit to any company or individual to **measure** the power consumption of its tech services and get this data in a convenient form, sending it through any monitoring or data analysis toolchain.

**Scaphandre** means *heavy* **diving suit** in [:fr:](https://fr.wikipedia.org/wiki/Scaphandre_%C3%A0_casque). It comes from the idea that tech related services often don't track their power consumption and thus don't expose it to their clients. Most of the time the reason is a presumed bad [ROI](https://en.wikipedia.org/wiki/Return_on_investment). Scaphandre makes, for tech providers and tech users, easier and cheaper to go under the surface to bring back the desired power consumption metrics, take better sustainability focused decisions, and then show the metrics to their clients to allow them to do the same.

This project was born from a deep sense of duty from tech workers. Please refer to the [why](https://hubblo-org.github.io/scaphandre-documentation/why.html) section to know more about its goals.

**Warning**: this is still a very early stage project. Any feedback or contribution will be highly appreciated. Please refer to the [contribution](https://hubblo-org.github.io/scaphandre-documentation/contributing.html) section.

![Fmt+Clippy](https://github.com/hubblo-org/scaphandre/workflows/Tests/badge.svg?branch=main)
[![](https://img.shields.io/crates/v/scaphandre.svg?maxAge=25920)](https://crates.io/crates/scaphandre)
<a href="https://gitter.im/hubblo-org/scaphandre?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge"><img src="https://badges.gitter.im/Join%20Chat.svg"></a>

Join us on [Gitter](https://gitter.im/hubblo-org/scaphandre) or [Matrix](https://app.element.io/#/room/#hubblo-org_scaphandre:gitter.im) !

---

## ✨ Features

- measuring power/energy consumed on **bare metal hosts**
- measuring power/energy consumed of **qemu/kvm virtual machines** from the host
- **exposing** power/energy metrics of a virtual machine, to allow **manipulating those metrics in the VM** as if it was a bare metal machine (relies on hypervisor features)
- exposing metrics as a **[prometheus](https://prometheus.io) (HTTP) exporter**
- sending metrics in push mode to a **[prometheus](https://prometheus.io) [Push Gateway](https://github.com/prometheus/pushgateway)**
- sending metrics to **[riemann](http://riemann.io/)**
- sending metrics to **[Warp10](http://warp10.io/)**
- works on **[kubernetes](https://kubernetes.io/)**
- storing power consumption metrics in a **JSON** file
- showing basic power consumption metrics **in the terminal**
- operating systems supported so far : **Gnu/Linux**, **Windows 10, 11 and Server 2016/2019/2022**
- packages available for **RHEL 8 and 9, Debian 11 and 12 and Windows**, also **NixOS** (community support)

Here is an example dashboard built thanks to scaphandre: [https://metrics.hubblo.org](https://metrics.hubblo.org).

<a href="https://metrics.hubblo.org"><img src="https://github.com/hubblo-org/scaphandre/raw/main/docs_src/grafana-dash-scaphandre.cleaned.png" width="800"></a>

## 📄 How to ... ?

You'll find everything you may want to know about scaphandre in the [documentation](https://hubblo-org.github.io/scaphandre-documentation), like:

- 🏁 [Getting started](https://hubblo-org.github.io/scaphandre-documentation/tutorials/getting_started.html)
- 💻 [Installation & compilation on GNU/Linux](https://hubblo-org.github.io/scaphandre-documentation/tutorials/installation-linux.html) or [on Windows](https://hubblo-org.github.io/scaphandre-documentation/tutorials/installation-windows.html)
- 👁️ [Give a virtual machine access to its power consumption metrics, and break the opacity of being on the computer of someone else](https://hubblo-org.github.io/scaphandre-documentation/how-to_guides/propagate-metrics-hypervisor-to-vm_qemu-kvm.html)
- 🎉 [Contributing guide](https://hubblo-org.github.io/scaphandre-documentation/contributing.html)
- [And much more](https://hubblo-org.github.io/scaphandre-documentation)

If you are only interested in the code documentation [here it is](https://docs.rs/scaphandre).

## 📅 Roadmap

The ongoing roadmap can be seen [here](https://github.com/hubblo-org/scaphandre/projects/1). Feature requests are welcome, please join us.

## ⚖️  Footprint

In opposition to its name, scaphandre aims to be as light and clean as possible. One of the main focus areas of the project is to come as close as possible to a 0 overhead, both about resources consumption and power consumption.

## 🙏 Sponsoring

If you like this project and would like to provide financial help, here's our [sponsoring page](https://github.com/sponsors/hubblo-org).
Thanks a lot for considering it !