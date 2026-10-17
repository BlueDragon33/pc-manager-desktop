# P20 disposable Windows development preflight. Read-only; NEVER launches NSIS.
# This is not Authenticode, a trusted update feed, or a Production policy.
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][string]$BaselineInstallerPath,
  [Parameter(Mandatory = $true)][string]$CandidateInstallerPath,
  [Parameter(Mandatory = $true)][string]$ExpectedCandidateVersion,
  [Parameter(Mandatory = $true)][AllowEmptyString()][string]$ExpectedCandidateSha256
)
$ErrorActionPreference = 'Stop'
function Reject([string]$reason) {
  throw "P20_REJECT:$reason"
}
function ReadVersion([string]$path) {
  $filename = [IO.Path]::GetFileName($path)
  if ($filename -cnotmatch '^PC Manager Desktop_([0-9]+[.][0-9]+[.][0-9]+)_x64-setup[.]exe$') {
    Reject 'BAD_FILE_NAME'
  }
  try { return [version]::Parse($Matches[1]) }
  catch { Reject 'BAD_FILE_VERSION' }
}
if (-not (Test-Path -LiteralPath $BaselineInstallerPath -PathType Leaf)) {
  Reject 'NO_BASELINE'
}
if (-not (Test-Path -LiteralPath $CandidateInstallerPath -PathType Leaf)) {
  Reject 'NO_CANDIDATE'
}
if ($ExpectedCandidateSha256 -cnotmatch '^[a-fA-F0-9]{64}$') {
  Reject 'BAD_HASH_FORMAT'
}
if ($ExpectedCandidateVersion -cnotmatch '^[0-9]+[.][0-9]+[.][0-9]+$') {
  Reject 'BAD_EXPECTED_VERSION'
}
try { $expectedVersion = [version]::Parse($ExpectedCandidateVersion) }
catch { Reject 'BAD_EXPECTED_VERSION' }
$baselineVersion = ReadVersion $BaselineInstallerPath
$candidateVersion = ReadVersion $CandidateInstallerPath
if ($candidateVersion -ne $expectedVersion) {
  Reject 'EXPECTED_VERSION_MISMATCH'
}
if ($candidateVersion -le $baselineVersion) {
  Reject 'VERSION_NOT_INCREASING'
}
$baselineSha = (Get-FileHash -LiteralPath $BaselineInstallerPath -Algorithm SHA256).Hash.ToLowerInvariant()
$candidateSha = (Get-FileHash -LiteralPath $CandidateInstallerPath -Algorithm SHA256).Hash.ToLowerInvariant()
if ($candidateSha -ne $ExpectedCandidateSha256.ToLowerInvariant()) {
  Reject 'CHECKSUM_MISMATCH'
}
if ($candidateSha -eq $baselineSha) {
  Reject 'IDENTICAL_PAYLOAD'
}
[pscustomobject]@{
  baselineVersion = $baselineVersion.ToString()
  candidateVersion = $candidateVersion.ToString()
  baselineSha256 = $baselineSha
  candidateSha256 = $candidateSha
  scope = 'unsigned-development-only'
  signedTrustVerified = $false
  installerInvoked = $false
}
