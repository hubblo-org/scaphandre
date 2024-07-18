# Qemu exporter

Computes energy consumption metrics for each Qemu/KVM virtual machine found on the host.
Exposes those metrics as filetrees compatible with the [powercap_rapl sensor](sensor-powercap_rapl.md).

Note that this is still experimental. Metrics are already considered trustworthy, but there are discussions and tests to be performed about the acceptable ways to share the data with the guests/vms. Any feedback or thoughts about this are welcome. Please refer to the [contributing section](../contributing.md).

## Usage

1. Run the scaphandre with the qemu exporter on your bare metal hypervisor machine:

		scaphandre qemu # this is suitable for a test, please run it as a systemd service for a production setup

2. Default is to expose virtual machines metrics in `/var/lib/libvirt/scaphandre/${DOMAIN_NAME}` with `DOMAIN_NAME` being the libvirt domain name of the virtual machine.
First create a tmpfs mount point to isolate metrics for that virtual machine:

		mount -t tmpfs tmpfs_DOMAIN_NAME /var/lib/libvirt/scaphandre/DOMAIN_NAME -o size=10m

3. Ensure you expose the content of this folder to the virtual machine by having this configuration in the xml configuration of the domain:
    ```
    		<filesystem type='mount' accessmode='passthrough'>
    	      <driver type='virtiofs'/>
    	      <source dir='/var/lib/libvirt/scaphandre/DOMAIN_NAME'/>
    	      <target dir='scaphandre'/>
    		  <readonly />
    	    </filesystem>
    ```
    You can edit the vm properties using `sudo virsh edit <DOMAIN_NAME>` using your usual editor. But it is more convenient to use virtual-manager, as explained in the following screenshots.

    *It also helps to define the correct syntax which probably depends from the qemu version. You can check that the above configuration is slightly different form the one below*.

    a. Right click in the hardware menu:
    ![virtualmgr00](images/virtualmgr00.png)

    b. Enter the following parameters:
    ![virtualmgr01](images/virtualmgr01.png)

    c. XML generated as a result:
    ![virtualmgr02](images/virtualmgr02.png)

    4. Ensure the VM has been started once the configuration is applied, then mount the filesystem on the VM/guest:

    		mount -t 9p -o trans=virtio scaphandre /var/scaphandre

    5. Still in the guest, run scaphandre in VM mode with the default sensor:

    		scaphandre --vm prometheus

    6. Collect your virtual machine specific power usage metrics. (requesting http://VM_IP:8080/metrics in this example, using the prometheus exporter)
  
## Usage (PROXMOX VIM)
   
1. Run scaphandre with the qemu exporter on your bare metal hypervisor machine:
	
		scaphandre qemu # this is suitable for a test, please run it as a systemd service for a production setup

2. Default is to expose virtual machines metrics in `/var/lib/libvirt/scaphandre/${VM_NAME}` with `VM_NAME` being the name of the virtual machine (VM).
First, add the following line at the end of the `/etc/pve/qemu-server/${<VM_ID}.conf` file, with `VM_ID` being the ID that PROXMOX has assigned your VM.

		args: -fsdev local,security_model=passthrough,id=fsdev0,path=/var/lib/libvirt/scaphandre/${VM_NAME} -device virtio-9p-pci,id=fs0,fsdev=fsdev0,mount_tag=${VM_NAME}

3. If you perform this file change with the VM running, you need to restart it for this modification to take effect.
4. In the guest (VM), mount the required directory in Read-Only mode:

		mount -t 9p -o ro,trans=virtio,version=9p2000.L ${VM_NAME} /var/scaphandre

5. Still in the guest, run scaphandre in VM mode with the default sensor:

		scaphandre --vm prometheus

6. Collect your virtual machine specific power usage metrics. (requesting http://${VM_IP}:8080/metrics in this example, using the prometheus exporter)
