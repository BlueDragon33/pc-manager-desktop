param(
  [Parameter(Mandatory = $true)]
  [string]$ArtifactPath,
  [Parameter(Mandatory = $true)]
  [string]$Version,
  [Parameter(Mandatory = $true)]
  [ValidateSet("dev", "beta", "stable")]
  [string]$Channel,
  [Parameter(Mandatory = $true)]
  [string]$ArtifactUrl,
  [Parameter(Mandatory = $true)]
  [string]$OutputPath,
  [string]$ReleaseNotesPath = "",
  [switch]$Mandatory
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $ArtifactPath -PathType Leaf)) {
  throw "Release artifact does not exist: $ArtifactPath"
}
if ($ArtifactUrl -notmatch '^https://') {
  throw "ArtifactUrl must use HTTPS."
}
if ($Version -notmatch '^\d+\.\d+\.\d+([+-][0-9A-Za-z.-]+)?$') {
  throw "Version must be a semantic version."
}

$notes = @()
if ($ReleaseNotesPath -and (Test-Path -LiteralPath $ReleaseNotesPath -PathType Leaf)) {
  $notes = @(Get-Content -LiteralPath $ReleaseNotesPath |
    Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
    ForEach-Object { $_.Trim() })
}

$hash = (Get-FileHash -LiteralPath $ArtifactPath -Algorithm SHA256).Hash.ToLowerInvariant()
$metadata = [ordered]@{
  version = $Version
  channel = $Channel
  mandatory = [bool]$Mandatory
  sha256 = $hash
  artifactUrl = $ArtifactUrl
  releaseNotes = $notes
  publishedAt = (Get-Date).ToUniversalTime().ToString("o")
  platform = "windows"
  architecture = "x86_64"
}

$parent = Split-Path -Parent $OutputPath
if ($parent) {
  New-Item -ItemType Directory -Path $parent -Force | Out-Null
}
$metadata | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $OutputPath -Encoding UTF8
Write-Host "Generated updater metadata at $OutputPath with SHA-256 $hash."
