# P22 Windows CI-only forensic assertion. Reproduce tauri-bundler 2.10.1
# in-place NSIS type token patch IN MEMORY, never modify an executable.
# Bundler restores the original file after packing; a post-bundle hash
# is therefore NOT the hash of the temporary NSIS input bytes.
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][string]$UnpatchedExecutablePath,
  [Parameter(Mandatory = $true)][string]$ExpectedUnpatchedSha256,
  [Parameter(Mandatory = $true)][string]$ExpectedNsisPayloadSha256
)
$ErrorActionPreference = 'Stop'
function Reject([string]$reason) { throw "P22_PATCH_REJECT:$reason" }
if (-not (Test-Path -LiteralPath $UnpatchedExecutablePath -PathType Leaf)) { Reject 'NO_SOURCE_BINARY' }
if ($ExpectedUnpatchedSha256 -cnotmatch '^[A-Fa-f0-9]{64}$' -or
    $ExpectedNsisPayloadSha256 -cnotmatch '^[A-Fa-f0-9]{64}$') { Reject 'INVALID_EXPECTED_HASH' }
$original = [IO.File]::ReadAllBytes($UnpatchedExecutablePath)
$actualOriginalSha = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($original)).ToLowerInvariant()
if ($actualOriginalSha -cne $ExpectedUnpatchedSha256.ToLowerInvariant()) { Reject 'SOURCE_HASH_MISMATCH' }
$marker = '__TAURI_BUNDLE_TYPE_VAR_UNK'
$nsis = '__TAURI_BUNDLE_TYPE_VAR_NSS'
$token = [Text.Encoding]::ASCII.GetBytes($marker)
$replacement = [Text.Encoding]::ASCII.GetBytes($nsis)
if ($token.Length -ne $replacement.Length) { Reject 'TOKEN_LENGTH' }
$latin = [Text.Encoding]::Latin1.GetString($original)
$offset = $latin.IndexOf($marker, [StringComparison]::Ordinal)
if ($offset -lt 0) { Reject 'UNPATCHED_TOKEN_ABSENT' }
if ($latin.IndexOf($marker, $offset + 1, [StringComparison]::Ordinal) -ge 0) {
  Reject 'AMBIGUOUS_UNPATCHED_TOKEN'
}
[Array]::Copy($replacement, 0, $original, $offset, $replacement.Length)
$patchedSha = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($original)).ToLowerInvariant()
if ($patchedSha -cne $ExpectedNsisPayloadSha256.ToLowerInvariant()) { Reject 'PATCHED_PAYLOAD_MISMATCH' }
[pscustomobject]@{
  schema='pc-manager.tauri-nsis-binary-patch/v1'
  unpatchedSha256=$actualOriginalSha
  expectedTemporaryNsisInputSha256=$patchedSha
  actualNsisPayloadSha256=$ExpectedNsisPayloadSha256.ToLowerInvariant()
  tokenOffsetBytes=$offset
  patch='__TAURI_BUNDLE_TYPE_VAR_UNK -> __TAURI_BUNDLE_TYPE_VAR_NSS'
  algorithm='tauri-bundler-2.10.1-in-memory-token-replacement'
  executableModifiedOnDisk=$false
  signatureTrustVerified=$false
  success=$true
}
