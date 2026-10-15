# Registers a local-only, per-user Windows URL protocol for the installed PC Manager.
# This helper does not run a server, grant elevation, or allow command parameters.
# Explicit invocation by a local user is required. Production installers will
# need separately verified lifecycle/ownership and rollback before automation.
[CmdletBinding(DefaultParameterSetName = 'Register')]
param(
    [Parameter(Mandatory = $true)]
    [ValidateNotNullOrEmpty()]
    [string]$ExecutablePath,
    [Parameter(ParameterSetName = 'Unregister', Mandatory = $true)]
    [switch]$Unregister
)

$ErrorActionPreference = 'Stop'
$schemeKey = 'HKCU:\Software\Classes\pcmanager'
$commandKey = Join-Path $schemeKey 'shell\open\command'
$exe = [System.IO.Path]::GetFullPath($ExecutablePath)

if ([System.IO.Path]::GetFileName($exe) -ne 'PC Manager Desktop.exe') {
    throw 'Only the installed PC Manager Desktop.exe may own pcmanager://.'
}

$expectedCommand = '"{0}" "%1"' -f $exe
$previousCommand = $null
if (Test-Path -LiteralPath $commandKey) {
    $previousCommand = (Get-Item -LiteralPath $commandKey).GetValue('')
}

if ($previousCommand -and
    -not [string]::Equals($previousCommand, $expectedCommand, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'pcmanager:// already belongs to a different executable. Refusing to overwrite it.'
}

if ($Unregister) {
    if (-not $previousCommand) {
        Write-Output 'pcmanager:// is not registered for this account.'
        exit 0
    }
    Remove-Item -LiteralPath $schemeKey -Recurse -Force
    Write-Output 'PC Manager per-user URL protocol unregistered.'
    exit 0
}

if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
    throw 'PC Manager Desktop.exe not found; install the Windows application first.'
}

# No global HKLM registration; this is intentionally current-user scoped.
New-Item -Path $schemeKey -Force | Out-Null
Set-Item -LiteralPath $schemeKey -Value 'URL:PC Manager Offline App'
New-ItemProperty -LiteralPath $schemeKey -Name 'URL Protocol' -Value '' -PropertyType String -Force | Out-Null
New-Item -Path $commandKey -Force | Out-Null
Set-Item -LiteralPath $commandKey -Value $expectedCommand

$stored = (Get-Item -LiteralPath $commandKey).GetValue('')
if ($stored -ne $expectedCommand) {
    throw 'Windows protocol registration readback did not match the expected command.'
}
Write-Output 'Registered pcmanager://open for the installed PC Manager (current Windows account only).'
