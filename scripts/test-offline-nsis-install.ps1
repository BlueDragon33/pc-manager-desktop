# Disposable runner only. Verifies the actual Windows NSIS installer lifecycle.
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [string]$InstallerPath,
  [string]$RuntimeEvidencePath,
  [string]$RepairEvidencePath,
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
if ($RepairEvidencePath -and (-not $RuntimeEvidencePath -or -not $SourceRevision)) {
  throw 'NSIS repair needs an earlier real native runtime run and exact source revision.'
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
  if ($RepairEvidencePath) {
    # No signed-upgrade claim. Repair the *same* unsigned version only on the
    # disposable Windows runner after the app has been closed by UIA smoke.
    Invoke-Installer -path $InstallerPath -operation 'NSIS same-version repair'
    if (-not (Test-Path -LiteralPath $exe -PathType Leaf) -or
        -not (Test-Path -LiteralPath $commandKey) -or
        ([string](Get-Item -LiteralPath $commandKey).GetValue('')) -ne $actual) {
      throw 'NSIS repair changed the installed executable or owned protocol handler.'
    }
    & "$PSScriptRoot/test-offline-desktop-runtime.ps1" -ExecutablePath $exe -EvidencePath $RepairEvidencePath -SourceRevision $SourceRevision -VerifyStoredStateOnly
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
