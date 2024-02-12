# https://github.com/dansmith
#
$source = "scaphandre"


$wid=[System.Security.Principal.WindowsIdentity]::GetCurrent()
$prp=new-object System.Security.Principal.WindowsPrincipal($wid)
$adm=[System.Security.Principal.WindowsBuiltInRole]::Administrator
$IsAdmin=$prp.IsInRole($adm)

if($IsAdmin -eq $false)
{
    [System.Reflection.Assembly]::LoadWithPartialName(“System.Windows.Forms”)
    [Windows.Forms.MessageBox]::Show(“Please run this as an Administrator”,
        “Not Administrator”,
        [Windows.Forms.MessageBoxButtons]::OK,
        [Windows.Forms.MessageBoxIcon]::Information)
    exit
}


if ([System.Diagnostics.EventLog]::SourceExists($source) -eq $false)
{
    [System.Diagnostics.EventLog]::CreateEventSource($source, "Application")
    
    [System.Reflection.Assembly]::LoadWithPartialName(“System.Windows.Forms”)
    [Windows.Forms.MessageBox]::Show(“Event log created successfully”,
        “Complete”,
        [Windows.Forms.MessageBoxButtons]::OK,
        [Windows.Forms.MessageBoxIcon]::Information)
}
else
{
    [System.Reflection.Assembly]::LoadWithPartialName(“System.Windows.Forms”)
    [Windows.Forms.MessageBox]::Show(“Event log already exists”,
        “Complete”,
        [Windows.Forms.MessageBoxButtons]::OK,
        [Windows.Forms.MessageBoxIcon]::Information)

}