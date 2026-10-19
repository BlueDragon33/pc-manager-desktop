# P22 CI evidence-truth guard. sourceRevision is the INSTALLED app's source;
# checkoutRevision is the test-harness checkout, which may be different.
[CmdletBinding()]
param(
  [Parameter(Mandatory=$true)][string]$EvidencePath,
  [Parameter(Mandatory=$true)][string]$ExpectedSourceRevision,
  [Parameter(Mandatory=$true)][string]$ExpectedCheckoutRevision
)
$ErrorActionPreference = 'Stop'
function Reject([string]$reason) { throw "P22_REVISION_REJECT:$reason" }
$shaPattern = '^[a-fA-F0-9]{40}$'
if ($ExpectedSourceRevision -cnotmatch $shaPattern -or
    $ExpectedCheckoutRevision -cnotmatch $shaPattern) { Reject 'INVALID_EXPECTED_SHA' }
if (-not (Test-Path -LiteralPath $EvidencePath -PathType Leaf)) { Reject 'NO_EVIDENCE' }
try {
  $proof = Get-Content -LiteralPath $EvidencePath -Raw -ErrorAction Stop | ConvertFrom-Json -ErrorAction Stop
} catch {
  Reject 'INVALID_JSON'
}
if ($null -eq $proof -or $proof -is [array] -or
    $null -eq $proof.PSObject -or
    'sourceRevision' -cnotin $proof.PSObject.Properties.Name -or
    'checkoutRevision' -cnotin $proof.PSObject.Properties.Name -or
    'success' -cnotin $proof.PSObject.Properties.Name) {
  Reject 'MISSING_REQUIRED_FIELDS'
}
if ($proof.success -cne $true) { Reject 'EVIDENCE_NOT_SUCCESSFUL' }
$source = [string]$proof.sourceRevision
$checkout = [string]$proof.checkoutRevision
if ($source -cnotmatch $shaPattern -or $checkout -cnotmatch $shaPattern) {
  Reject 'INVALID_RECORDED_SHA'
}
if ($source -cne $ExpectedSourceRevision) { Reject 'SOURCE_MISMATCH' }
if ($checkout -cne $ExpectedCheckoutRevision) { Reject 'CHECKOUT_MISMATCH' }
Write-Host "PASS P22 evidence attribution: source=$source checkout=$checkout file=$([IO.Path]::GetFileName($EvidencePath))"
