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

Then add this filesystem configuration block inside the `<devices></devices>` block:

    <filesystem type='mount' accessmode='passthrough'>
        <driver type='virtiofs'/>
        <source dir='/var/lib/libvirt/scaphandre/DOMAIN_NAME'/>
        <target dir='scaphandre'/>
        <readonly />
    </filesystem>

Save and (re)start the virtual machine.

If you get this error: "error: unsupported configuration: 'virtiofs' requires shared memory", you might add this configuration section to the `<domain>` section.

    <memoryBacking>
      <source type='memfd'/>
      <access mode='shared'/>
    </memoryBacking>

Then connect to the virtual machine and mount the filesystem:

     mount -t 9p -o trans=virtio scaphandre /var/scaphandre

You can now run scaphandre to export the metrics with the exporter of your choice (here prometheus):

     scaphandre --vm prometheus

## How to expose metrics in PROXMOX Virtual Environment
   
Run scaphandre with the qemu exporter on your bare metal hypervisor machine:
	
	scaphandre qemu

The Qemu exporter will expose virtual machine metrics in `/var/lib/libvirt/scaphandre/${VM_NAME}` with `VM_NAME` being the name of the virtual machine (VM).
Add the following line at the end of the `/etc/pve/qemu-server/${<VM_ID}.conf` file, with `VM_ID` being the ID that PROXMOX has assigned your VM.

	args: -fsdev local,security_model=passthrough,id=fsdev0,path=/var/lib/libvirt/scaphandre/${VM_NAME} -device virtio-9p-pci,id=fs0,fsdev=fsdev0,mount_tag=${VM_NAME}

If you perform this file change with the VM running, you need to restart it for this modification to take effect.
In the guest (VM), mount the required directory in Read-Only mode:

	mount -t 9p -o ro,trans=virtio,version=9p2000.L ${VM_NAME} /var/scaphandre

Still in the guest, run scaphandre in VM mode with the default sensor:

	scaphandre --vm prometheus
 
Please refer to the [qemu exporter](../references/exporter-qemu.md) reference for more details.

**Note:** This how to is only suitable for a "manual" use case. For automated systems like openstack, some more work needs to be done to make the integration of those steps easier.
