# Fast, disposable P22 provenance guard negative/positive cases; no NSIS, no shell privilege.
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$guard = Join-Path $PSScriptRoot 'assert-p22-evidence-revisions.ps1'
$root = Join-Path $env:RUNNER_TEMP ("pc-manager-p22-revision-fixtures-" + [guid]::NewGuid().ToString('N'))
$path = Join-Path $root 'source-evidence.json'
$baseline = '83645a56987666144020dbdc85b3eec12815d003'
$candidate = '9c7290999bdd0395eb8f7bebc3a22550e27db3ae'
function Fixture($source,$checkout,$successful=$true) {
  @{ sourceRevision=$source; checkoutRevision=$checkout; success=$successful } |
    ConvertTo-Json -Compress | Set-Content -LiteralPath $path -Encoding utf8
}
function Expect-Reject([string]$name,[string]$reason,[string]$source,[string]$checkout) {
  $observed = $null
  try { & $guard -EvidencePath $path -ExpectedSourceRevision $source -ExpectedCheckoutRevision $checkout }
  catch { $observed = $_.Exception.Message }
  if (-not $observed -or -not $observed.Contains("P22_REVISION_REJECT:$reason")) {
    throw "P22 guard negative case $name did not fail closed for $reason. Got: $observed"
  }
  Write-Host "PASS P22 revision negative $name : $reason"
}
try {
  New-Item -ItemType Directory -Path $root -Force | Out-Null
  Fixture $baseline $candidate
  & $guard -EvidencePath $path -ExpectedSourceRevision $baseline -ExpectedCheckoutRevision $candidate
  Fixture $candidate $candidate
  & $guard -EvidencePath $path -ExpectedSourceRevision $candidate -ExpectedCheckoutRevision $candidate
  Fixture $candidate $candidate
  Expect-Reject 'baseline-labeled-as-candidate' 'SOURCE_MISMATCH' $baseline $candidate
  Fixture $baseline $baseline
  Expect-Reject 'harness-checkout-misattributed' 'CHECKOUT_MISMATCH' $baseline $candidate
  Fixture $baseline $candidate $false
  Expect-Reject 'unverified-runtime' 'EVIDENCE_NOT_SUCCESSFUL' $baseline $candidate
  @{ checkoutRevision=$candidate; success=$true } | ConvertTo-Json | Set-Content -LiteralPath $path -Encoding utf8
  Expect-Reject 'missing-source' 'MISSING_REQUIRED_FIELDS' $baseline $candidate
  Set-Content -LiteralPath $path -Value '{invalid-json' -Encoding utf8
  Expect-Reject 'malformed-json' 'INVALID_JSON' $baseline $candidate
  Write-Host 'PASS P22 evidence revisions: two positive cases and four fail-closed negative cases.'
} finally {
  if (Test-Path -LiteralPath $root) { Remove-Item -LiteralPath $root -Force -Recurse -ErrorAction SilentlyContinue }
}
