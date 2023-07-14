# Install Scaphandre on Windows

**!! Warning: This is a first testing version of the package and installation procedure.**
**!! A new version is on its way with proper driver signature and Windows service proper management.**

## Using the installer

In this first itration of the package, you'll need to enable Test Mode on Windows prior to proceed to this installation, then reboot. (Next version will have an officially signed version of the driver, so this won't be ncessaerry anymore.)

    bcdedit.exe -set TESTSIGNING ON
    bcdedit.exe -set nointegritychecks on

The installer will ensure that test mode is enabled and fail otherwise, but activation of test mode **and a reboot** is needed before anyway.

Then download the [package](https://scaphandre.s3.fr-par.scw.cloud/x86_64/scaphandre_0.5.0_installer.exe) and install it **as an administrator**.

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