# Propagate power consumption metrics from hypervisor to virtual machines (Qemu/KVM)

## Introduction

A major pain point in measuring power consumption is doing so inside a virtual machine. A virtual machine usually doesn't have access to power metrics.

Scaphandre aims at solving that by enabling a communication between a scaphandre instance **on the hypervisor/bare metal machine** and **another one** running **on the virtual machine**.
The scaphandre agent on the hypervisor will **compute the metrics meaningful for that virtual machine** and the one **on the VM access those metrics** to allow its user/administrator to use the data as if they had access to power metrics in the first place (as if they were on a bare metal machine).

This allows to break opacity in a virtualization context, if you have access to the hypervisor, or in a  public cloud context if the provider uses scaphandre on its hypervisors.

<img src="../virtu.cleaned.png" width="650"/>

## How to

This is working on Qemu/KVM hypervisors only.

The idea is to run the agent on the hypervisor, with the [qemu exporter](../references/exporter-qemu.md):

    scaphandre qemu

More examples for a production ready setup will be added soon (systemd service, docker container, ...). If you think the documentation needs a refresh now, please [contribute](https://github.com/hubblo-org/scaphandre/pulls) :)
    
For each virtual machine you want to give access to its metrics, create a [tmpfs](https://en.wikipedia.org/wiki/Tmpfs) mountpoint:

     mount -t tmpfs tmpfs_DOMAIN_NAME /var/lib/libvirt/scaphandre/DOMAIN_NAME -o size=5m

In the definition of the virtual machine (here we are using libvirt), ensure you have a filesystem configuration to give access to the mountpoint:

    virsh edit DOMAIN_NAME

Then add:

    <filesystem type='mount' accessmode='passthrough'>
        <driver type='virtiofs'/>
        <source dir='/var/lib/libvirt/scaphandre/DOMAIN_NAME'/>
        <target dir='scaphandre'/>
        <readonly />
    </filesystem>

Save and (re)start the virtual machine.

Then connect to the virtual machine and mount the filesystem:

     mount -t 9p -o trans=virtio scaphandre /var/scaphandre

You can now run scaphandre to export the metrics with the exporter of your choice (here prometheus):

     scaphandre --vm prometheus

Please refer to the [qemu exporter](docs/exporters/qemu.md) reference for more details.

**Note:** This how to is only suitable for a "manual" use case. For all automated systems like openstack or proxmox, some more work needs to be done to make the integration of those steps easier.
