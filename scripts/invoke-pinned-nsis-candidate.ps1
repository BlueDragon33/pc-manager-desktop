# P21: CI-only, unsigned NSIS handoff. Keep an exclusive-writer-denying
# read handle open from final preflight through Start-Process completion.
# This is NOT Authenticode, a signed updater, nor a complete TOCTOU proof.
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][string]$BaselineInstallerPath,
  [Parameter(Mandatory = $true)][string]$CandidateInstallerPath,
  [Parameter(Mandatory = $true)][string]$ExpectedCandidateVersion,
  [Parameter(Mandatory = $true)][string]$ExpectedCandidateSha256
)
$ErrorActionPreference = 'Stop'
function Reject([string]$reason) { throw "P21_REJECT:$reason" }
if (-not (Test-Path -LiteralPath $CandidateInstallerPath -PathType Leaf)) {
  Reject 'NO_CANDIDATE'
}
# FileShare.Read denies existing/future write or delete handles for the
# open file. A writer already holding the candidate makes this fail closed.
$lock = $null
try {
  $lock = [IO.File]::Open(
    $CandidateInstallerPath,
    [IO.FileMode]::Open,
    [IO.FileAccess]::Read,
    [IO.FileShare]::Read
  )
} catch {
  Reject 'CANDIDATE_NOT_LOCKABLE'
}
try {
  # Verify the actual bytes before launching under the same live lock.
  $proof = & "$PSScriptRoot/assert-nsis-upgrade-preflight.ps1" `
    -BaselineInstallerPath $BaselineInstallerPath `
    -CandidateInstallerPath $CandidateInstallerPath `
    -ExpectedCandidateVersion $ExpectedCandidateVersion `
    -ExpectedCandidateSha256 $ExpectedCandidateSha256
  if ($null -eq $proof -or $proof.signedTrustVerified -ne $false -or
      $proof.installerInvoked -ne $false) {
    Reject 'UNTRUSTED_PREFLIGHT_RESULT'
  }
  $hasher = [Security.Cryptography.SHA256]::Create()
  try {
    $lock.Position = 0
    $actualSha = [Convert]::ToHexString($hasher.ComputeHash($lock)).ToLowerInvariant()
  } finally {
    $hasher.Dispose()
  }
  if ($actualSha -cne $ExpectedCandidateSha256.ToLowerInvariant() -or
      $actualSha -cne $proof.candidateSha256) {
    Reject 'LOCKED_BYTES_MISMATCH'
  }
  # Only the approved CI development candidate is executed; no elevation.
  $process = Start-Process -FilePath $CandidateInstallerPath -ArgumentList '/S' -Wait -PassThru
  if ($process.ExitCode -ne 0) {
    Reject "INSTALLER_FAILED_$($process.ExitCode)"
  }
  [pscustomobject]@{
    scope = 'unsigned-disposable-windows-ci'
    candidateSha256 = $actualSha
    candidateVersion = $proof.candidateVersion
    writeDenyHandleHeldThroughExit = $true
    signerVerified = $false
    productionAuthorized = $false
    installerInvoked = $true
  }
} finally {
  if ($null -ne $lock) { $lock.Dispose() }
}
