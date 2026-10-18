# CI-only independent readback of executable payload inside an unsigned NSIS archive.
# 7-Zip extracts bytes without running NSIS; never implies signer verification.
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][string]$InstallerPath
)
$ErrorActionPreference = 'Stop'
if (-not (Test-Path -LiteralPath $InstallerPath -PathType Leaf)) {
  throw 'P22_PAYLOAD:NO_INSTALLER'
}
$sevenZip = Get-Command '7z.exe' -ErrorAction SilentlyContinue
if (-not $sevenZip) {
  $fallback = Join-Path $env:ProgramFiles '7-Zip\7z.exe'
  if (Test-Path -LiteralPath $fallback -PathType Leaf) {
    $sevenZip = Get-Item -LiteralPath $fallback
  }
}
if (-not $sevenZip) { throw 'P22_PAYLOAD:NO_7ZIP_EXTRACTOR' }
$sevenZipPath = if ($sevenZip.Source) { $sevenZip.Source } else { $sevenZip.FullName }
$root = Join-Path $env:RUNNER_TEMP ("pc-manager-p22-nsis-readback-" + [guid]::NewGuid().ToString('N'))
try {
  New-Item -ItemType Directory -Path $root -Force | Out-Null
  & $sevenZipPath x '-y' '-bd' "-o$root" $InstallerPath | Out-Null
  if ($LASTEXITCODE -ne 0) { throw "P22_PAYLOAD:EXTRACTION_FAILED:$LASTEXITCODE" }
  $files = @(Get-ChildItem -LiteralPath $root -Recurse -File -Filter 'pc-manager-desktop.exe' -ErrorAction Stop)
  if ($files.Count -eq 0) {
    # NSIS may embed a single nested compressed application payload.
    $archives = @(Get-ChildItem -LiteralPath $root -Recurse -File -ErrorAction Stop |
      Where-Object { $_.Extension -in @('.7z', '.zip') })
    if ($archives.Count -ne 1) { throw "P22_PAYLOAD:NOT_ONE_NESTED_ARCHIVE:$($archives.Count)" }
    $nestedRoot = Join-Path $root '_nested'
    New-Item -ItemType Directory -Path $nestedRoot -Force | Out-Null
    & $sevenZipPath x '-y' '-bd' "-o$nestedRoot" $archives[0].FullName | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'P22_PAYLOAD:NESTED_EXTRACTION_FAILED' }
    $files = @(Get-ChildItem -LiteralPath $nestedRoot -Recurse -File -Filter 'pc-manager-desktop.exe' -ErrorAction Stop)
  }
  if ($files.Count -ne 1) { throw "P22_PAYLOAD:AMBIGUOUS_EXECUTABLE_COUNT:$($files.Count)" }
  $sha = (Get-FileHash -LiteralPath $files[0].FullName -Algorithm SHA256).Hash.ToLowerInvariant()
  [pscustomobject]@{
    installerSha256 = (Get-FileHash -LiteralPath $InstallerPath -Algorithm SHA256).Hash.ToLowerInvariant()
    payloadExecutableSha256 = $sha
    payloadExecutableLength = [long]$files[0].Length
    extractionMethod = '7zip-nsis-read-only'
    executableCandidates = 1
    installerInvoked = $false
    authenticodeVerified = $false
  }
} finally {
  if (Test-Path -LiteralPath $root) {
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
  }
}
