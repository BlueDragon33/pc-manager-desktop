# Disposable runner only. Verifies the actual Windows NSIS installer lifecycle.
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [string]$InstallerPath,
  [string]$RuntimeEvidencePath,
  [string]$RepairEvidencePath,
  [string]$UpgradeInstallerPath,
  [string]$UpgradeEvidencePath,
  [string]$PreflightEvidencePath,
  [string]$ExpectedUpgradeSha256,
  [string]$SourceRevision,
  [string]$BaselineSourceRevision,
  [string]$CandidateSourceRevision,
  [string]$ExpectedBaselineExeSha256,
  [string]$ExpectedCandidateExeSha256,
  [string]$ExpectedUpgradeMarker
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
  if ($file -notmatch '_(\d+\.\d+\.\d+)_x64-setup[.]exe$') {
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
if ([bool]$UpgradeInstallerPath -ne [bool]$UpgradeEvidencePath) {
  throw 'Upgrade installer and separate upgrade evidence must be supplied together.'
}
if ($UpgradeInstallerPath -and (-not $RepairEvidencePath -or -not $SourceRevision)) {
  throw 'Cross-version pilot requires successful same-version repair and exact revision.'
}
if ($UpgradeInstallerPath -and ((-not $PreflightEvidencePath) -or (-not $ExpectedUpgradeSha256))) {
  throw 'P20 requires pinned expected candidate SHA256 and separate preflight evidence.'
}
if ($UpgradeInstallerPath) {
  # P22: the installed app source and compiled bytes must have independent provenance.
  if ($CandidateSourceRevision -cne $SourceRevision -or
      $BaselineSourceRevision -cnotmatch '^[a-fA-F0-9]{40}$' -or
      $CandidateSourceRevision -cnotmatch '^[a-fA-F0-9]{40}$' -or
      $BaselineSourceRevision -ceq $CandidateSourceRevision -or
      $ExpectedBaselineExeSha256 -cnotmatch '^[a-fA-F0-9]{64}$' -or
      $ExpectedCandidateExeSha256 -cnotmatch '^[a-fA-F0-9]{64}$' -or
      $ExpectedBaselineExeSha256 -ceq $ExpectedCandidateExeSha256 -or
      $ExpectedUpgradeMarker -cne 'Offline-first state on this device') {
    throw 'P22 real two-source SHA, compiled executable hash or UI marker missing.'
  }
  if ((git rev-parse HEAD).Trim() -cne $CandidateSourceRevision) {
    throw 'P22 harness checkout does not match the exact candidate source.'
  }
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
  $installedBaselineHash = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
  $baselineCompiledBytesMatchInstalled = ($installedBaselineHash -ceq $ExpectedBaselineExeSha256.ToLowerInvariant())
  if ($UpgradeInstallerPath) {
    Write-Host "P22 baseline builtExeSha256=$($ExpectedBaselineExeSha256.ToLowerInvariant()); installedExeSha256=$installedBaselineHash; bytesEqual=$baselineCompiledBytesMatchInstalled"
    if (-not $baselineCompiledBytesMatchInstalled) {
      Write-Warning 'P22 NSIS installed bytes differ from post-build executable; record separately; do not claim exact compiled-byte equivalence.'
    }
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
    & "$PSScriptRoot/test-offline-desktop-runtime.ps1" -ExecutablePath $exe -EvidencePath $RepairEvidencePath -SourceRevision $SourceRevision -VerifyStoredStateOnly -ForbiddenUpgradeMarker $ExpectedUpgradeMarker
    if ($UpgradeInstallerPath -and (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant() -cne $installedBaselineHash) {
      throw 'P22 same-version repair altered installed baseline executable bytes.'
    }
  }
  if ($UpgradeInstallerPath) {
    # P20 DENY tests run with the installed old version and persisted state.
    # They invoke no NSIS executable. A positive preflight is required before
    # the only higher-version installer call below.
    & "$PSScriptRoot/test-nsis-upgrade-preflight.ps1" -BaselineInstallerPath $InstallerPath -CandidateInstallerPath $UpgradeInstallerPath -ExpectedCandidateSha256 $ExpectedUpgradeSha256 -ExecutablePath $exe -ExpectedOwnedHandler $actual -EvidencePath $PreflightEvidencePath -SourceRevision $SourceRevision
    $preflightProof = Get-Content -LiteralPath $PreflightEvidencePath -Raw | ConvertFrom-Json
    if (-not $preflightProof.success -or $preflightProof.checks.Count -ne 11) {
      throw 'P20/P21 negative and positive preflight evidence missing or incomplete.'
    }
    # P21: hold a write-denying read handle for final P20 validation,
    # rehash and the complete child process lifetime. Never bypass this
    # with a bare Start-Process when the handoff cannot be locked.
    # P19 remains a same-source, unsigned VERSION-LABEL pilot, not rollback.
    $beforeExeHash = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
    $handoff = & "$PSScriptRoot/invoke-pinned-nsis-candidate.ps1" -BaselineInstallerPath $InstallerPath -CandidateInstallerPath $UpgradeInstallerPath -ExpectedCandidateVersion $candidateVersion.ToString() -ExpectedCandidateSha256 $ExpectedUpgradeSha256
    if (-not $handoff.writeDenyHandleHeldThroughExit -or
        $handoff.candidateSha256 -cne $ExpectedUpgradeSha256.ToLowerInvariant() -or
        $handoff.signerVerified -ne $false -or $handoff.productionAuthorized -ne $false) {
      throw 'P21 locked NSIS handoff evidence invalid.'
    }
    if (-not (Test-Path -LiteralPath $exe -PathType Leaf) -or
        -not (Test-Path -LiteralPath $commandKey) -or
        ([string](Get-Item -LiteralPath $commandKey).GetValue('')) -ne $actual) {
      throw 'Version-transition installer failed to retain executable and owned URL handler.'
    }
    $candidateInstalledHash = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
    $candidateCompiledBytesMatchInstalled = ($candidateInstalledHash -ceq $ExpectedCandidateExeSha256.ToLowerInvariant())
    Write-Host "P22 candidate builtExeSha256=$($ExpectedCandidateExeSha256.ToLowerInvariant()); installedExeSha256=$candidateInstalledHash; bytesEqual=$candidateCompiledBytesMatchInstalled"
    if ($candidateInstalledHash -ceq $beforeExeHash) {
      throw 'P22 installed candidate is byte-identical to the installed baseline.'
    }
    if (-not $candidateCompiledBytesMatchInstalled) {
      Write-Warning 'P22 NSIS installed bytes differ from post-build executable; no exact compiled-byte equivalence claim.'
    }
    & "$PSScriptRoot/test-offline-desktop-runtime.ps1" -ExecutablePath $exe -EvidencePath $UpgradeEvidencePath -SourceRevision $SourceRevision -VerifyStoredStateOnly -ExpectedUpgradeMarker $ExpectedUpgradeMarker
    $proof = Get-Content -LiteralPath $UpgradeEvidencePath -Raw | ConvertFrom-Json
    if (-not $proof.success -or $proof.verificationMode -ne 'unsigned-same-version-repair') {
      throw 'Upgrade application state was not actually verified by native UIA.'
    }
    $afterExeHash = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
    $proof.verificationMode = 'unsigned-two-source-package-transition'
    $proof | Add-Member -NotePropertyName packageTransition -NotePropertyValue ([ordered]@{
      fromVersion = $baselineVersion.ToString()
      toVersion = $candidateVersion.ToString()
      fromInstallerSha256 = $baselineHash
      toInstallerSha256 = $candidateHash
      executableBytesChanged = ($beforeExeHash -ne $afterExeHash)
      fromSourceRevision = $BaselineSourceRevision
      toSourceRevision = $CandidateSourceRevision
      baselineBuiltExecutableSha256 = $ExpectedBaselineExeSha256.ToLowerInvariant()
      candidateBuiltExecutableSha256 = $ExpectedCandidateExeSha256.ToLowerInvariant()
      baselineInstalledExecutableSha256 = $installedBaselineHash
      candidateInstalledExecutableSha256 = $candidateInstalledHash
      baselineBuiltBytesEqualInstalled = $baselineCompiledBytesMatchInstalled
      candidateBuiltBytesEqualInstalled = $candidateCompiledBytesMatchInstalled
      installedExecutableBytesChanged = ($installedBaselineHash -cne $candidateInstalledHash)
      changedApplicationSource = $true
      sameSourceCode = $false
      candidateUiMarkerObserved = $true
      signedUpgrade = $false
      automaticRollbackVerified = $false
    })
    $proof | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $UpgradeEvidencePath -Encoding utf8
    Write-Output "PASS P22 two-source-transition : $BaselineSourceRevision -> $CandidateSourceRevision; unsigned $baselineVersion -> $candidateVersion preserved native WebView2 local state and the new source UI marker."
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
