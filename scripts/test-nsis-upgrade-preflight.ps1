# P20 negative Windows preflight on the actually installed P18 state.
# No installer is called by this script. Evidence must not imply rollback testing.
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][string]$BaselineInstallerPath,
  [Parameter(Mandatory = $true)][string]$CandidateInstallerPath,
  [Parameter(Mandatory = $true)][string]$ExpectedCandidateSha256,
  [Parameter(Mandatory = $true)][string]$ExecutablePath,
  [Parameter(Mandatory = $true)][string]$ExpectedOwnedHandler,
  [Parameter(Mandatory = $true)][string]$EvidencePath,
  [Parameter(Mandatory = $true)][string]$SourceRevision
)
$ErrorActionPreference = 'Stop'
$checks = [System.Collections.Generic.List[object]]::new()
$success = $false
$preflight = Join-Path $PSScriptRoot 'assert-nsis-upgrade-preflight.ps1'
$handlerKey = 'HKCU:\Software\Classes\pcmanager\shell\open\command'
$malformedCandidate = $null
$samePayloadCandidate = $null
function Evidence([string]$case,[string]$status,[string]$detail) {
  $checks.Add([pscustomobject]@{case=$case;status=$status;detail=$detail})
  Write-Host "$status P20 $case : $detail"
}
function Check-Installed-Identity {
  if (-not (Test-Path -LiteralPath $ExecutablePath -PathType Leaf)) {
    throw 'Installed executable disappeared during rejected preflight.'
  }
  if (-not (Test-Path -LiteralPath $handlerKey)) {
    throw 'Owned URL handler disappeared during rejected preflight.'
  }
  $actualHandler = [string](Get-Item -LiteralPath $handlerKey).GetValue('')
  if ($actualHandler -cne $ExpectedOwnedHandler) {
    throw 'Owned URL handler changed during rejected preflight.'
  }
  $actualHash = (Get-FileHash -LiteralPath $ExecutablePath -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($actualHash -cne $script:installedExeSha) {
    throw 'Installed executable changed during rejected preflight.'
  }
}
function Expect-Rejection([string]$case,[string]$reason,[string]$path,[string]$version,[string]$hash) {
  $observed = $null
  try {
    $null = & $preflight -BaselineInstallerPath $BaselineInstallerPath -CandidateInstallerPath $path -ExpectedCandidateVersion $version -ExpectedCandidateSha256 $hash
  } catch {
    $observed = $_.Exception.Message
  }
  if ($null -eq $observed -or -not $observed.Contains("P20_REJECT:$reason")) {
    throw "Negative case $case was NOT rejected for the expected reason ($reason); observed: $observed"
  }
  Check-Installed-Identity
  Evidence $case 'PASS' "Rejected before any installer call: $reason; installed exe SHA and HKCU handler unchanged."
}
try {
  if (-not (Test-Path -LiteralPath $preflight -PathType Leaf)) { throw 'Preflight script missing.' }
  if (-not (Test-Path -LiteralPath $ExecutablePath -PathType Leaf)) { throw 'Expected installed exe missing.' }
  if (-not (Test-Path -LiteralPath $CandidateInstallerPath -PathType Leaf)) { throw 'Expected candidate missing.' }
  if (-not (Test-Path -LiteralPath $BaselineInstallerPath -PathType Leaf)) { throw 'Expected baseline missing.' }
  $script:installedExeSha = (Get-FileHash -LiteralPath $ExecutablePath -Algorithm SHA256).Hash.ToLowerInvariant()
  Check-Installed-Identity
  $baselineVersion = [regex]::Match([IO.Path]::GetFileName($BaselineInstallerPath),'_([0-9]+[.][0-9]+[.][0-9]+)_x64-setup[.]exe$').Groups[1].Value
  $candidateVersion = [regex]::Match([IO.Path]::GetFileName($CandidateInstallerPath),'_([0-9]+[.][0-9]+[.][0-9]+)_x64-setup[.]exe$').Groups[1].Value
  if (-not $baselineVersion -or -not $candidateVersion) { throw 'Version-named test fixtures unavailable.' }
  $baselineHash = (Get-FileHash -LiteralPath $BaselineInstallerPath -Algorithm SHA256).Hash.ToLowerInvariant()
  $tempRoot = Join-Path $env:RUNNER_TEMP "pc-manager-p20-negatives-$PID"
  New-Item -ItemType Directory -Path $tempRoot -Force | Out-Null
  $malformedCandidate = Join-Path $tempRoot 'invalid-package.exe'
  Copy-Item -LiteralPath $CandidateInstallerPath -Destination $malformedCandidate
  $sameRoot = Join-Path $tempRoot 'same-bytes'
  New-Item -ItemType Directory -Path $sameRoot -Force | Out-Null
  $samePayloadCandidate = Join-Path $sameRoot ([IO.Path]::GetFileName($CandidateInstallerPath))
  Copy-Item -LiteralPath $BaselineInstallerPath -Destination $samePayloadCandidate
  Expect-Rejection 'missing-file' 'NO_CANDIDATE' (Join-Path $tempRoot 'not-found.exe') $candidateVersion $ExpectedCandidateSha256
  Expect-Rejection 'malformed-name' 'BAD_FILE_NAME' $malformedCandidate $candidateVersion $ExpectedCandidateSha256
  Expect-Rejection 'same-version' 'VERSION_NOT_INCREASING' $BaselineInstallerPath $baselineVersion $baselineHash
  Expect-Rejection 'wrong-version-expectation' 'EXPECTED_VERSION_MISMATCH' $CandidateInstallerPath $baselineVersion $ExpectedCandidateSha256
  Expect-Rejection 'wrong-checksum' 'CHECKSUM_MISMATCH' $CandidateInstallerPath $candidateVersion ('0' * 64)
  Expect-Rejection 'missing-checksum' 'BAD_HASH_FORMAT' $CandidateInstallerPath $candidateVersion ''
  Expect-Rejection 'identical-payload' 'IDENTICAL_PAYLOAD' $samePayloadCandidate $candidateVersion $baselineHash
  # P21: an actively writable candidate must be denied before any NSIS call.
  # This is a disposable copy, never the positive staged candidate.
  $writableDir = Join-Path $tempRoot 'writer-open'
  New-Item -ItemType Directory -Path $writableDir -Force | Out-Null
  $writableCandidate = Join-Path $writableDir ([IO.Path]::GetFileName($CandidateInstallerPath))
  Copy-Item -LiteralPath $CandidateInstallerPath -Destination $writableCandidate
  $writer = [IO.File]::Open($writableCandidate, [IO.FileMode]::Open, [IO.FileAccess]::ReadWrite, [IO.FileShare]::ReadWrite)
  $observedHandoff = $null
  try {
    try {
      $null = & "$PSScriptRoot/invoke-pinned-nsis-candidate.ps1" -BaselineInstallerPath $BaselineInstallerPath -CandidateInstallerPath $writableCandidate -ExpectedCandidateVersion $candidateVersion -ExpectedCandidateSha256 $ExpectedCandidateSha256
    } catch {
      $observedHandoff = $_.Exception.Message
    }
  } finally {
    $writer.Dispose()
  }
  if ($null -eq $observedHandoff -or -not $observedHandoff.Contains('P21_REJECT:CANDIDATE_NOT_LOCKABLE')) {
    throw "P21 open-writer handoff was not denied as expected: $observedHandoff"
  }
  Check-Installed-Identity
  Evidence 'open-writer-handoff' 'PASS' 'Active writable handle rejected by pinned handoff before NSIS; installed state unchanged.'
  Check-Installed-Identity
  Evidence 'post-rejection-identity' 'PASS' 'Installed executable SHA and owned HKCU handler unchanged after all eight denied candidates.'
  & "$PSScriptRoot/test-offline-desktop-runtime.ps1" -ExecutablePath $ExecutablePath -EvidencePath ($EvidencePath + '.uia.json') -SourceRevision $SourceRevision -VerifyStoredStateOnly -StoredStateContext rejected-upgrade-preflight
  $uiProof = Get-Content -LiteralPath ($EvidencePath + '.uia.json') -Raw | ConvertFrom-Json
  if (-not $uiProof.success -or $uiProof.verificationMode -ne 'rejected-upgrade-preflight') { throw 'Real WebView2 post-rejection state readback failed or misclassified.' }
  Evidence 'post-rejection-native-state' 'PASS' 'Real installed Windows UIA retained managed OFF, dark appearance, compact sidebar and Health report.'
  $positive = & $preflight -BaselineInstallerPath $BaselineInstallerPath -CandidateInstallerPath $CandidateInstallerPath -ExpectedCandidateVersion $candidateVersion -ExpectedCandidateSha256 $ExpectedCandidateSha256
  if ($positive.candidateSha256 -cne $ExpectedCandidateSha256.ToLowerInvariant() -or $positive.signedTrustVerified -ne $false) {
    throw 'Positive preflight identity/trust classification mismatch.'
  }
  Evidence 'positive-preflight' 'PASS' 'Actual version-increasing unsigned candidate accepted only with exact expected SHA256.'
  $success = $true
} catch {
  Evidence 'preflight' 'FAIL' $_.Exception.Message
  throw
} finally {
  if ($malformedCandidate) {
    $root = Split-Path -Parent $malformedCandidate
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
  }
  $parent = Split-Path -Parent $EvidencePath
  if ($parent) { New-Item -ItemType Directory -Path $parent -Force | Out-Null }
  [pscustomobject]@{
    schema='pc-manager.nsis-preflight/v1'
    sourceRevision=$SourceRevision
    checkoutRevision=(git rev-parse HEAD)
    networkScope='APP_AND_WEBVIEW2_OUTBOUND_BLOCKED_FOR_NATIVE_READBACK'
    signedTrustVerified=$false
    installerLaunchedByNegativeTests=$false
    failedInstallRollbackVerified=$false
    userWindowsAccepted=$false
    success=$success
    checks=@($checks.ToArray())
  } | ConvertTo-Json -Depth 7 | Set-Content -LiteralPath $EvidencePath -Encoding utf8
}
