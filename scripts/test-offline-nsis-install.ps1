# Disposable runner only. Verifies the actual Windows NSIS installer lifecycle.
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [string]$InstallerPath,
  [string]$RuntimeEvidencePath,
  [string]$SourceRevision
)
$ErrorActionPreference = 'Stop'
$scheme = 'HKCU:\Software\Classes\pcmanager'
$commandKey = Join-Path $scheme 'shell\open\command'
$exe = $null
$installed = $false

function Invoke-Installer([string]$path, [string]$operation) {
  $process = Start-Process -FilePath $path -ArgumentList '/S' -Wait -PassThru
  if ($process.ExitCode -ne 0) { throw "$operation returned $($process.ExitCode)" }
}

if (-not (Test-Path -LiteralPath $InstallerPath -PathType Leaf)) {
  throw 'NSIS Preview installer is absent.'
}
if (Test-Path -LiteralPath $scheme) {
  throw 'Runner is not disposable: pcmanager scheme exists, refusing to overwrite.'
}

try {
  $installed = $true
  Invoke-Installer -path $InstallerPath -operation 'NSIS installation'
  if (-not (Test-Path -LiteralPath $commandKey)) {
    throw 'Installer did not register the current-user offline URL scheme.'
  }
  $actual = [string](Get-Item -LiteralPath $commandKey).GetValue('')
  # Exact, argument-free executable. Untrusted URL must not reach process argv.
  if ($actual -notmatch '^"([^"]+\\pc-manager-desktop\.exe)"$') {
    throw "Unsafe or incorrect handler: $actual"
  }
  $exe = $Matches[1]
  if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
    throw 'Registered application executable not installed.'
  }
  $uninstaller = Join-Path (Split-Path -Parent $exe) 'uninstall.exe'
  if (-not (Test-Path -LiteralPath $uninstaller -PathType Leaf)) {
    throw 'Uninstaller was not packaged.'
  }
  if ($RuntimeEvidencePath) {
    if (-not $SourceRevision) { throw 'Exact source revision missing.' }
    & "$PSScriptRoot/test-offline-desktop-runtime.ps1" -ExecutablePath $exe -EvidencePath $RuntimeEvidencePath -SourceRevision $SourceRevision
  }
  Invoke-Installer -path $uninstaller -operation 'NSIS uninstall'
  $installed = $false
  if (Test-Path -LiteralPath $scheme) {
    throw 'Uninstaller did not remove its own registered URL handler.'
  }
  Write-Output 'PASS signed-off scope: actual unsigned NSIS install/owned protocol/uninstall.'
} finally {
  if ($installed -and $exe -and (Test-Path -LiteralPath $exe -PathType Leaf)) {
    $uninstaller = Join-Path (Split-Path -Parent $exe) 'uninstall.exe'
    if (Test-Path -LiteralPath $uninstaller -PathType Leaf) {
      try { Invoke-Installer -path $uninstaller -operation 'Cleanup' }
      catch { Write-Warning $_ }
    }
  }
}
