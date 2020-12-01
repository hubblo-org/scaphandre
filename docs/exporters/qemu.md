# Qemu exporter

Computes energy consumption metrics for each Qemu/KVM virtual machine found on the host.
Exposes those metrics as filetrees compatible with the [powercap_rapl sensor](../sensors/powercap_rapl.md).

## Usage

1. Run the scaphandre with the qemu exporter on your bare metal hypervisor machine:

	scaphandre qemu # this is suitable for a test, please run it as a systemd service for a production setup

2. Default is to expose virtual machines metrics in `/var/lib/libvirt/scaphandre/${DOMAIN_NAME}` with `VM_NAME` being the libvirt domain name of the virtual machine. Ensure you expose the content of this folder to the virtual machine by having this configuration in the xml configuration of the domain:

	<filesystem type='mount' accessmode='passthrough'>
      <driver type='virtiofs'/>
      <source dir='/var/lib/libvirt/scaphandre/DOMAIN_NAME'/>
      <target dir='scaphandre'/>
	  <readonly />
    </filesystem>

3. Ensure the VM has been started once the configuration is applied, then mount the filesystem on the VM/guest:

	mount -t 9p -o trans=virtio scaphandre /var/scaphandre

4. Still in the guest, run scaphandre in VM mode with the default sensor:

	scaphandre --vm prometheus

5. Collect your virtual machine specific power usage metrics. (getting http://VM_IP:8080/metrics in this example)

As always exporter's options can be displayed with `-h`:

	scaphandre qemu -h