 ; Script generated by the Inno Setup Script Wizard.
; SEE THE DOCUMENTATION FOR DETAILS ON CREATING INNO SETUP SCRIPT FILES!

#define MyAppName "scaphandre"
#define MyAppVersion "0.5.0"
#define MyAppPublisher "Hubblo"
#define MyAppURL "https://hubblo-org.github.io/scaphandre-documentation"
#define MyAppExeName "scaphandre.exe"
#define SystemFolder "C:\Windows\System32"
#define System64Folder "C:\Windows\SysWOW64"

[Setup]
; NOTE: The value of AppId uniquely identifies this application. Do not use the same AppId value in installers for other applications.
; (To generate a new GUID, click Tools | Generate GUID inside the IDE.)
AppId={{7DB7B851-1DD2-4FF5-BFC7-282FEBA3B28D}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
;AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
LicenseFile=../../LICENSE
; Uncomment the following line to run in non administrative install mode (install for current user only.)
;PrivilegesRequired=lowest
OutputBaseFilename={#MyAppName}_installer
Compression=lzma
SolidCompression=yes
WizardStyle=modern
Uninstallable=yes
SetupIconFile=../../docs_src/scaphandre.ico

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
Source: "../../target/release/{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "../../DriverLoader.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "../../ScaphandreDrv.inf"; DestDir: "{app}"; Flags: ignoreversion
; Source: "../../ScaphandreDrv.sys"; DestDir: "{#SystemFolder}";
; Source: "../../ScaphandreDrv.sys"; DestDir: "{#System64Folder}";
Source: "../../ScaphandreDrv.sys"; DestDir: "{app}";
Source: "../../ScaphandreDrv.cat"; DestDir: "{app}";
; Source: "../../ScaphandreDrv.cat"; DestDir: "{#SystemFolder}";
; Source: "../../ScaphandreDrv.cat"; DestDir: "{#System64Folder}";
Source: "C:\Program Files (x86)\Windows Kits\10\Tools\10.0.22621.0\x64\devcon.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\certmgr.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "../../README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "../../CHANGELOG.md"; DestDir: "{app}"; Flags: ignoreversion
; NOTE: Don't use "Flags: ignoreversion" on any shared system files

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"

[Run]
Filename: "C:\windows\System32\WindowsPowershell\v1.0\powershell.exe"; Parameters: "Import-Certificate -FilePath {app}\ScaphandreDrvTest.cer -CertStoreLocation Cert:\LocalMachine\Root"; Description: "Register test certificate"; Flags: waituntilidle shellexec
Filename: "{app}/devcon.exe"; Parameters: "install {app}\ScaphandreDrv.inf root\SCAPHANDREDRV"; Description: "Install Driver"; Flags: waituntilidle 
Filename: "{app}/devcon.exe"; Parameters: "enable {app}\ScaphandreDrv.inf root\SCAPHANDREDRV"; Description: "Enable Driver"; Flags: waituntilidle
Filename: "{app}/DriverLoader.exe"; Parameters: "install"; WorkingDir: "{app}"; Description: "Install Driver Service";
Filename: "{app}/DriverLoader.exe"; Parameters: "start"; WorkingDir: "{app}"; Description: "Start Driver Service"; 
; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}";

[UninstallRun]
Filename: "{app}/DriverLoader.exe"; Parameters: "stop"; WorkingDir: "{app}"; RunOnceId: "StopService";
Filename: "{app}/DriverLoader.exe"; Parameters: "remove"; WorkingDir: "{app}"; RunOnceId: "RemoveService";
Filename: "{app}/devcon.exe"; Parameters: "disable ScaphandreDrv"; RunOnceId: "DisableDrier";
Filename: "{app}/devcon.exe"; Parameters: "remove ScaphandreDrv"; RunOnceId: "RemoveService";


