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
  [string]$ExpectedBaselinePreBundleSha256,
  [string]$ExpectedCandidatePreBundleSha256,
  [string]$ExpectedUpgradeMarker,
  [string]$BinaryProvenanceEvidencePath
)
$ErrorActionPreference = 'Stop'
$scheme = 'HKCU:\Software\Classes\pcmanager'
$commandKey = Join-Path $scheme 'shell\open\command'
$exe = $null
$installed = $false
$baselinePayload = $null
$candidatePayload = $null
$installedBaselineHash = $null
$candidateInstalledHash = $null
$binaryProvenanceVerified = $false

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
if ($UpgradeInstallerPath -and (-not $BinaryProvenanceEvidencePath)) {
  throw 'P22 requires independent three-way binary provenance artifact.'
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
      $ExpectedBaselinePreBundleSha256 -cnotmatch '^[a-fA-F0-9]{64}$' -or
      $ExpectedCandidatePreBundleSha256 -cnotmatch '^[a-fA-F0-9]{64}$' -or
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
  if ($UpgradeInstallerPath) {
    # P22: inspect both actual NSIS archives independent of the post-build path
    # and independent of installation. 7-Zip only extracts; it never runs NSIS.
    $baselinePayload = & "$PSScriptRoot/read-nsis-executable-payload.ps1" -InstallerPath $InstallerPath
    $candidatePayload = & "$PSScriptRoot/read-nsis-executable-payload.ps1" -InstallerPath $UpgradeInstallerPath
    if ($baselinePayload.installerSha256 -cne $baselineHash -or
        $candidatePayload.installerSha256 -cne $candidateHash -or
        $baselinePayload.payloadExecutableSha256 -ceq $candidatePayload.payloadExecutableSha256) {
      throw 'P22 NSIS payload provenance mismatched staged installer or source versions.'
    }
    Write-Host "P22 NSIS package readback baselinePayload=$($baselinePayload.payloadExecutableSha256) candidatePayload=$($candidatePayload.payloadExecutableSha256)"
  }
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
  # P22 now passes the actual target/release hash captured AFTER Tauri bundle.
  # This is not the unbundled compiler output; that has its own SHA input.
  $baselinePostBundleMatchesInstalled = ($installedBaselineHash -ceq $ExpectedBaselineExeSha256.ToLowerInvariant())
  if ($UpgradeInstallerPath -and
      $installedBaselineHash -cne $baselinePayload.payloadExecutableSha256) {
    throw 'P22 baseline installed executable is not the executable extracted from its NSIS installer.'
  }
  if ($UpgradeInstallerPath) {
    Write-Host "P22 baseline unbundledSha256=$($ExpectedBaselinePreBundleSha256.ToLowerInvariant()); postBundleSha256=$($ExpectedBaselineExeSha256.ToLowerInvariant()); nsisPayloadSha256=$($baselinePayload.payloadExecutableSha256); installedSha256=$installedBaselineHash"
    if (-not $baselinePostBundleMatchesInstalled) {
      throw 'P22 baseline Tauri post-bundle executable does not match NSIS-installed binary.'
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
    $candidatePostBundleMatchesInstalled = ($candidateInstalledHash -ceq $ExpectedCandidateExeSha256.ToLowerInvariant())
    if ($candidateInstalledHash -cne $candidatePayload.payloadExecutableSha256) {
      throw 'P22 upgraded installed executable is not the executable extracted from candidate NSIS.'
    }
    $binaryProvenanceVerified = $true
    Write-Host 'PASS P22 NSIS payload-to-installed SHA256 exact for both source versions.'
    Write-Host "P22 candidate unbundledSha256=$($ExpectedCandidatePreBundleSha256.ToLowerInvariant()); postBundleSha256=$($ExpectedCandidateExeSha256.ToLowerInvariant()); nsisPayloadSha256=$($candidatePayload.payloadExecutableSha256); installedSha256=$candidateInstalledHash"
    if ($candidateInstalledHash -ceq $beforeExeHash) {
      throw 'P22 installed candidate is byte-identical to the installed baseline.'
    }
    if (-not $candidatePostBundleMatchesInstalled) {
      throw 'P22 candidate Tauri post-bundle executable does not match NSIS-installed binary.'
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
      baselinePreBundleExecutableSha256 = $ExpectedBaselinePreBundleSha256.ToLowerInvariant()
      candidatePreBundleExecutableSha256 = $ExpectedCandidatePreBundleSha256.ToLowerInvariant()
      baselinePostBundleExecutableSha256 = $ExpectedBaselineExeSha256.ToLowerInvariant()
      candidatePostBundleExecutableSha256 = $ExpectedCandidateExeSha256.ToLowerInvariant()
      baselineMutatedDuringBundle = ($ExpectedBaselinePreBundleSha256.ToLowerInvariant() -cne $ExpectedBaselineExeSha256.ToLowerInvariant())
      candidateMutatedDuringBundle = ($ExpectedCandidatePreBundleSha256.ToLowerInvariant() -cne $ExpectedCandidateExeSha256.ToLowerInvariant())
      baselineInstalledExecutableSha256 = $installedBaselineHash
      candidateInstalledExecutableSha256 = $candidateInstalledHash
      baselineNsisPayloadSha256 = $baselinePayload.payloadExecutableSha256
      candidateNsisPayloadSha256 = $candidatePayload.payloadExecutableSha256
      nsisPayloadMatchesInstalled = $binaryProvenanceVerified
      postBuildToNsisMutationExplained = $false
      baselinePostBundleMatchesInstalled = $baselinePostBundleMatchesInstalled
      candidatePostBundleMatchesInstalled = $candidatePostBundleMatchesInstalled
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
  if ($UpgradeInstallerPath -and $BinaryProvenanceEvidencePath) {
    $parent = Split-Path -Parent $BinaryProvenanceEvidencePath
    if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
    [pscustomobject]@{
      schema = 'pc-manager.nsis-binary-provenance/v1'
      sourceRevision = $SourceRevision
      checkoutRevision = (git rev-parse HEAD).Trim()
      fromSourceRevision = $BaselineSourceRevision
      toSourceRevision = $CandidateSourceRevision
      baselinePreBundleSha256 = $ExpectedBaselinePreBundleSha256.ToLowerInvariant()
      candidatePreBundleSha256 = $ExpectedCandidatePreBundleSha256.ToLowerInvariant()
      baselinePostBundleSha256 = $ExpectedBaselineExeSha256.ToLowerInvariant()
      candidatePostBundleSha256 = $ExpectedCandidateExeSha256.ToLowerInvariant()
      baselineMutatedDuringBundle = ($ExpectedBaselinePreBundleSha256.ToLowerInvariant() -cne $ExpectedBaselineExeSha256.ToLowerInvariant())
      candidateMutatedDuringBundle = ($ExpectedCandidatePreBundleSha256.ToLowerInvariant() -cne $ExpectedCandidateExeSha256.ToLowerInvariant())
      baselineNsisPayloadSha256 = $(if ($baselinePayload) { $baselinePayload.payloadExecutableSha256 } else { $null })
      candidateNsisPayloadSha256 = $(if ($candidatePayload) { $candidatePayload.payloadExecutableSha256 } else { $null })
      baselineInstalledSha256 = $installedBaselineHash
      candidateInstalledSha256 = $candidateInstalledHash
      payloadMatchesInstalledBoth = $binaryProvenanceVerified
      postBundleNsisPayloadMatchesInstalled = $binaryProvenanceVerified
      bundleMutationLocationMeasured = $true
      tauriBinaryPatchingMechanismVerified = $false
      installerExecutionUnsigned = $true
      signedTrustVerified = $false
      humanWindowsAccepted = $false
      success = $binaryProvenanceVerified
    } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $BinaryProvenanceEvidencePath -Encoding utf8
  }
  if ($installed -and $exe -and (Test-Path -LiteralPath $exe -PathType Leaf)) {
    $uninstaller = Join-Path (Split-Path -Parent $exe) 'uninstall.exe'
    if (Test-Path -LiteralPath $uninstaller -PathType Leaf) {
      try { Invoke-Installer -path $uninstaller -operation 'Cleanup' }
      catch { Write-Warning $_ }
    }
  }
}
 -or
      $ExpectedBaselinePreBundleSha256 -cnotmatch '^[a-fA-F0-9]{64}
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
  if ($UpgradeInstallerPath) {
    # P22: inspect both actual NSIS archives independent of the post-build path
    # and independent of installation. 7-Zip only extracts; it never runs NSIS.
    $baselinePayload = & "$PSScriptRoot/read-nsis-executable-payload.ps1" -InstallerPath $InstallerPath
    $candidatePayload = & "$PSScriptRoot/read-nsis-executable-payload.ps1" -InstallerPath $UpgradeInstallerPath
    if ($baselinePayload.installerSha256 -cne $baselineHash -or
        $candidatePayload.installerSha256 -cne $candidateHash -or
        $baselinePayload.payloadExecutableSha256 -ceq $candidatePayload.payloadExecutableSha256) {
      throw 'P22 NSIS payload provenance mismatched staged installer or source versions.'
    }
    Write-Host "P22 NSIS package readback baselinePayload=$($baselinePayload.payloadExecutableSha256) candidatePayload=$($candidatePayload.payloadExecutableSha256)"
  }
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
  if ($UpgradeInstallerPath -and
      $installedBaselineHash -cne $baselinePayload.payloadExecutableSha256) {
    throw 'P22 baseline installed executable is not the executable extracted from its NSIS installer.'
  }
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
    if ($candidateInstalledHash -cne $candidatePayload.payloadExecutableSha256) {
      throw 'P22 upgraded installed executable is not the executable extracted from candidate NSIS.'
    }
    $binaryProvenanceVerified = $true
    Write-Host 'PASS P22 NSIS payload-to-installed SHA256 exact for both source versions.'
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
      baselineNsisPayloadSha256 = $baselinePayload.payloadExecutableSha256
      candidateNsisPayloadSha256 = $candidatePayload.payloadExecutableSha256
      nsisPayloadMatchesInstalled = $binaryProvenanceVerified
      postBuildToNsisMutationExplained = $false
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
  if ($UpgradeInstallerPath -and $BinaryProvenanceEvidencePath) {
    $parent = Split-Path -Parent $BinaryProvenanceEvidencePath
    if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
    [pscustomobject]@{
      schema = 'pc-manager.nsis-binary-provenance/v1'
      sourceRevision = $SourceRevision
      checkoutRevision = (git rev-parse HEAD).Trim()
      fromSourceRevision = $BaselineSourceRevision
      toSourceRevision = $CandidateSourceRevision
      baselinePostBuildSha256 = $ExpectedBaselineExeSha256.ToLowerInvariant()
      candidatePostBuildSha256 = $ExpectedCandidateExeSha256.ToLowerInvariant()
      baselineNsisPayloadSha256 = $(if ($baselinePayload) { $baselinePayload.payloadExecutableSha256 } else { $null })
      candidateNsisPayloadSha256 = $(if ($candidatePayload) { $candidatePayload.payloadExecutableSha256 } else { $null })
      baselineInstalledSha256 = $installedBaselineHash
      candidateInstalledSha256 = $candidateInstalledHash
      payloadMatchesInstalledBoth = $binaryProvenanceVerified
      postBuildToNsisMutationCauseVerified = $false
      installerExecutionUnsigned = $true
      signedTrustVerified = $false
      humanWindowsAccepted = $false
      success = $binaryProvenanceVerified
    } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $BinaryProvenanceEvidencePath -Encoding utf8
  }
  if ($installed -and $exe -and (Test-Path -LiteralPath $exe -PathType Leaf)) {
    $uninstaller = Join-Path (Split-Path -Parent $exe) 'uninstall.exe'
    if (Test-Path -LiteralPath $uninstaller -PathType Leaf) {
      try { Invoke-Installer -path $uninstaller -operation 'Cleanup' }
      catch { Write-Warning $_ }
    }
  }
}
 -or
      $ExpectedCandidatePreBundleSha256 -cnotmatch '^[a-fA-F0-9]{64}
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
  if ($UpgradeInstallerPath) {
    # P22: inspect both actual NSIS archives independent of the post-build path
    # and independent of installation. 7-Zip only extracts; it never runs NSIS.
    $baselinePayload = & "$PSScriptRoot/read-nsis-executable-payload.ps1" -InstallerPath $InstallerPath
    $candidatePayload = & "$PSScriptRoot/read-nsis-executable-payload.ps1" -InstallerPath $UpgradeInstallerPath
    if ($baselinePayload.installerSha256 -cne $baselineHash -or
        $candidatePayload.installerSha256 -cne $candidateHash -or
        $baselinePayload.payloadExecutableSha256 -ceq $candidatePayload.payloadExecutableSha256) {
      throw 'P22 NSIS payload provenance mismatched staged installer or source versions.'
    }
    Write-Host "P22 NSIS package readback baselinePayload=$($baselinePayload.payloadExecutableSha256) candidatePayload=$($candidatePayload.payloadExecutableSha256)"
  }
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
  if ($UpgradeInstallerPath -and
      $installedBaselineHash -cne $baselinePayload.payloadExecutableSha256) {
    throw 'P22 baseline installed executable is not the executable extracted from its NSIS installer.'
  }
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
    if ($candidateInstalledHash -cne $candidatePayload.payloadExecutableSha256) {
      throw 'P22 upgraded installed executable is not the executable extracted from candidate NSIS.'
    }
    $binaryProvenanceVerified = $true
    Write-Host 'PASS P22 NSIS payload-to-installed SHA256 exact for both source versions.'
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
      baselineNsisPayloadSha256 = $baselinePayload.payloadExecutableSha256
      candidateNsisPayloadSha256 = $candidatePayload.payloadExecutableSha256
      nsisPayloadMatchesInstalled = $binaryProvenanceVerified
      postBuildToNsisMutationExplained = $false
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
  if ($UpgradeInstallerPath -and $BinaryProvenanceEvidencePath) {
    $parent = Split-Path -Parent $BinaryProvenanceEvidencePath
    if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
    [pscustomobject]@{
      schema = 'pc-manager.nsis-binary-provenance/v1'
      sourceRevision = $SourceRevision
      checkoutRevision = (git rev-parse HEAD).Trim()
      fromSourceRevision = $BaselineSourceRevision
      toSourceRevision = $CandidateSourceRevision
      baselinePostBuildSha256 = $ExpectedBaselineExeSha256.ToLowerInvariant()
      candidatePostBuildSha256 = $ExpectedCandidateExeSha256.ToLowerInvariant()
      baselineNsisPayloadSha256 = $(if ($baselinePayload) { $baselinePayload.payloadExecutableSha256 } else { $null })
      candidateNsisPayloadSha256 = $(if ($candidatePayload) { $candidatePayload.payloadExecutableSha256 } else { $null })
      baselineInstalledSha256 = $installedBaselineHash
      candidateInstalledSha256 = $candidateInstalledHash
      payloadMatchesInstalledBoth = $binaryProvenanceVerified
      postBuildToNsisMutationCauseVerified = $false
      installerExecutionUnsigned = $true
      signedTrustVerified = $false
      humanWindowsAccepted = $false
      success = $binaryProvenanceVerified
    } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $BinaryProvenanceEvidencePath -Encoding utf8
  }
  if ($installed -and $exe -and (Test-Path -LiteralPath $exe -PathType Leaf)) {
    $uninstaller = Join-Path (Split-Path -Parent $exe) 'uninstall.exe'
    if (Test-Path -LiteralPath $uninstaller -PathType Leaf) {
      try { Invoke-Installer -path $uninstaller -operation 'Cleanup' }
      catch { Write-Warning $_ }
    }
  }
}
 -or
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
  if ($UpgradeInstallerPath) {
    # P22: inspect both actual NSIS archives independent of the post-build path
    # and independent of installation. 7-Zip only extracts; it never runs NSIS.
    $baselinePayload = & "$PSScriptRoot/read-nsis-executable-payload.ps1" -InstallerPath $InstallerPath
    $candidatePayload = & "$PSScriptRoot/read-nsis-executable-payload.ps1" -InstallerPath $UpgradeInstallerPath
    if ($baselinePayload.installerSha256 -cne $baselineHash -or
        $candidatePayload.installerSha256 -cne $candidateHash -or
        $baselinePayload.payloadExecutableSha256 -ceq $candidatePayload.payloadExecutableSha256) {
      throw 'P22 NSIS payload provenance mismatched staged installer or source versions.'
    }
    Write-Host "P22 NSIS package readback baselinePayload=$($baselinePayload.payloadExecutableSha256) candidatePayload=$($candidatePayload.payloadExecutableSha256)"
  }
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
  if ($UpgradeInstallerPath -and
      $installedBaselineHash -cne $baselinePayload.payloadExecutableSha256) {
    throw 'P22 baseline installed executable is not the executable extracted from its NSIS installer.'
  }
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
    if ($candidateInstalledHash -cne $candidatePayload.payloadExecutableSha256) {
      throw 'P22 upgraded installed executable is not the executable extracted from candidate NSIS.'
    }
    $binaryProvenanceVerified = $true
    Write-Host 'PASS P22 NSIS payload-to-installed SHA256 exact for both source versions.'
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
      baselineNsisPayloadSha256 = $baselinePayload.payloadExecutableSha256
      candidateNsisPayloadSha256 = $candidatePayload.payloadExecutableSha256
      nsisPayloadMatchesInstalled = $binaryProvenanceVerified
      postBuildToNsisMutationExplained = $false
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
  if ($UpgradeInstallerPath -and $BinaryProvenanceEvidencePath) {
    $parent = Split-Path -Parent $BinaryProvenanceEvidencePath
    if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
    [pscustomobject]@{
      schema = 'pc-manager.nsis-binary-provenance/v1'
      sourceRevision = $SourceRevision
      checkoutRevision = (git rev-parse HEAD).Trim()
      fromSourceRevision = $BaselineSourceRevision
      toSourceRevision = $CandidateSourceRevision
      baselinePostBuildSha256 = $ExpectedBaselineExeSha256.ToLowerInvariant()
      candidatePostBuildSha256 = $ExpectedCandidateExeSha256.ToLowerInvariant()
      baselineNsisPayloadSha256 = $(if ($baselinePayload) { $baselinePayload.payloadExecutableSha256 } else { $null })
      candidateNsisPayloadSha256 = $(if ($candidatePayload) { $candidatePayload.payloadExecutableSha256 } else { $null })
      baselineInstalledSha256 = $installedBaselineHash
      candidateInstalledSha256 = $candidateInstalledHash
      payloadMatchesInstalledBoth = $binaryProvenanceVerified
      postBuildToNsisMutationCauseVerified = $false
      installerExecutionUnsigned = $true
      signedTrustVerified = $false
      humanWindowsAccepted = $false
      success = $binaryProvenanceVerified
    } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $BinaryProvenanceEvidencePath -Encoding utf8
  }
  if ($installed -and $exe -and (Test-Path -LiteralPath $exe -PathType Leaf)) {
    $uninstaller = Join-Path (Split-Path -Parent $exe) 'uninstall.exe'
    if (Test-Path -LiteralPath $uninstaller -PathType Leaf) {
      try { Invoke-Installer -path $uninstaller -operation 'Cleanup' }
      catch { Write-Warning $_ }
    }
  }
}
