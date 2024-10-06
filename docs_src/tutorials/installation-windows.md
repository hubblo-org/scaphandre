# Install Scaphandre on Windows

**!! Warning: Windows version of Scaphandre is still in early stage. !!**

## Using the installer

Download the latest exe installer [from the release page](https://github.com/hubblo-org/scaphandre/releases) and install it **as an administrator**.

### Configuring a Windows service to run Scaphandre in the background

For example, to run the prometheus-push exporter in the background and target the Prometheus Push Gateway server with ip address `198.51.100.5` using HTTPS on port 443 and a step to send metrics of 45s, without checking the certificate of the push gateway (remove that option if you have a properly signed TLS certificate):

    sc.exe create Scaphandre binPath="C:\Program Files (x86)\scaphandre\scaphandre.exe prometheus-push -H 198.51.100.5 -s 45 -S https -p 443 --no-tls-check" DisplayName=Scaphandre start=auto

Ensure the service is started in Services.msc, start it by right clicking on it, then Start, otherwise.

To delete the service, you can do it in Services.msc, or: 

    sc.exe delete Scaphandre

### Using an installer including a development version of the driver

If you are running a development version of the installer (which probably means a development version of the [driver](https://github.com/hubblo-org/windows-rapl-driver/)), you'll need to enable Test Mode on Windows prior to proceed to this installation, then reboot.

    bcdedit.exe -set TESTSIGNING ON
    bcdedit.exe -set nointegritychecks on

Beware: in this case, activation of test mode **and a reboot** is needed before anyway.

Once installed, you should be able to run scaphandre from Powershell, by running :

    & 'C:\Program Files (x86)\scaphandre\scaphandre.exe' stdout

## Troubleshooting

An error such as

    scaphandre::sensors::msr_rapl: Failed to open device : HANDLE(-1)

means that the driver is not properly setup. Check it's state by running:

    driverquery /v | findstr capha

If there is not item returned, the installation of the driver encountered an issue.

If the service is STOPPED, there is also something wrong.

## Compilation

If you look for compiling Scaphandre and its driver yourself, see [Compilation for Windows](compilation-windows.md)