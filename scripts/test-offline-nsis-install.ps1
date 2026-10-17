# Disposable runner only. Verifies the actual Windows NSIS installer lifecycle.
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [string]$InstallerPath,
  [string]$RuntimeEvidencePath,
  [string]$RepairEvidencePath,
  [string]$UpgradeInstallerPath,
  [string]$UpgradeEvidencePath,
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

function Get-NsisBundleVersion([string]$path) {
  $file = [IO.Path]::GetFileName($path)
  if ($file -notmatch '_(\d+\.\d+\.\d+)_x64-setup\.exe

if (-not (Test-Path -LiteralPath $InstallerPath -PathType Leaf)) {
  throw 'NSIS Preview installer is absent.'
}
if (Test-Path -LiteralPath $scheme) {
  throw 'Runner is not disposable: pcmanager scheme exists, refusing to overwrite.'
}
if ($RepairEvidencePath -and (-not $RuntimeEvidencePath -or -not $SourceRevision)) {
  throw 'NSIS repair needs an earlier real native runtime run and exact source revision.'
}
if ([bool]$UpgradeInstallerPath -ne [bool]$UpgradeEvidencePath) {
  throw 'Upgrade installer and separate upgrade evidence must be supplied together.'
}
if ($UpgradeInstallerPath -and (-not $RepairEvidencePath -or -not $SourceRevision)) {
  throw 'Cross-version pilot requires successful same-version repair and exact revision.'
}
if ($UpgradeInstallerPath) {
  if (-not (Test-Path -LiteralPath $UpgradeInstallerPath -PathType Leaf)) {
    throw 'Unsigned development upgrade bundle is missing.'
  }
  $baselineVersion = Get-NsisBundleVersion $InstallerPath
  $candidateVersion = Get-NsisBundleVersion $UpgradeInstallerPath
  if ($candidateVersion -le $baselineVersion) {
    throw 'Refusing unsigned upgrade candidate without strictly greater NSIS version.'
  }
  $baselineHash = (Get-FileHash -LiteralPath $InstallerPath -Algorithm SHA256).Hash.ToLowerInvariant()
  $candidateHash = (Get-FileHash -LiteralPath $UpgradeInstallerPath -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($baselineHash -eq $candidateHash) {
    throw 'Baseline and candidate installers are byte-identical; no package transition proved.'
  }
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
  if ($UpgradeInstallerPath) {
    # Disposable-runner PILOT only: two distinct unsigned bundle versions
    # compiled from the same source, with a temporary version-only config.
    # Do not represent this as a signed/cross-code upgrade or rollback proof.
    $beforeExeHash = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
    Invoke-Installer -path $UpgradeInstallerPath -operation 'NSIS higher-version development upgrade'
    if (-not (Test-Path -LiteralPath $exe -PathType Leaf) -or
        -not (Test-Path -LiteralPath $commandKey) -or
        ([string](Get-Item -LiteralPath $commandKey).GetValue('')) -ne $actual) {
      throw 'Version-transition installer failed to retain executable and owned URL handler.'
    }
    & "$PSScriptRoot/test-offline-desktop-runtime.ps1" -ExecutablePath $exe -EvidencePath $UpgradeEvidencePath -SourceRevision $SourceRevision -VerifyStoredStateOnly
    $proof = Get-Content -LiteralPath $UpgradeEvidencePath -Raw | ConvertFrom-Json
    if (-not $proof.success -or $proof.verificationMode -ne 'unsigned-same-version-repair') {
      throw 'Upgrade application state was not actually verified by native UIA.'
    }
    $afterExeHash = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
    $proof.verificationMode = 'unsigned-version-only-package-transition'
    $proof | Add-Member -NotePropertyName packageTransition -NotePropertyValue ([ordered]@{
      fromVersion = $baselineVersion.ToString()
      toVersion = $candidateVersion.ToString()
      fromInstallerSha256 = $baselineHash
      toInstallerSha256 = $candidateHash
      executableBytesChanged = ($beforeExeHash -ne $afterExeHash)
      sameSourceCode = $true
      signedUpgrade = $false
      automaticRollbackVerified = $false
    })
    $proof | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $UpgradeEvidencePath -Encoding utf8
    Write-Output "PASS version-transition : unsigned $baselineVersion -> $candidateVersion preserved real WebView2 preferences and Health report."
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
) {
    throw "NSIS version naming is unverified: $file"
  }
  return [version]$Matches[1]
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
