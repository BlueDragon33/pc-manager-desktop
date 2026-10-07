param(
  [Parameter(Mandatory = $true)]
  [string]$ArtifactPath
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $ArtifactPath -PathType Leaf)) {
  throw "Signing target does not exist: $ArtifactPath"
}

if ($env:PC_MANAGER_ALLOW_UNSIGNED_PREVIEW -eq "1") {
  Write-Warning "UNSIGNED PREVIEW: Authenticode signing intentionally skipped."
  exit 0
}

if ([string]::IsNullOrWhiteSpace($env:WINDOWS_SIGNING_PFX_BASE64) -or
    [string]::IsNullOrWhiteSpace($env:WINDOWS_SIGNING_PFX_PASSWORD)) {
  throw "Production signing material is unavailable. Refusing to publish an unsigned Production artifact."
}

$signTool = (Get-Command signtool.exe -ErrorAction SilentlyContinue).Source
if (-not $signTool) {
  $programFilesX86 = [Environment]::GetFolderPath("ProgramFilesX86")
  $candidates = Get-ChildItem "$programFilesX86\Windows Kits\10\bin" -Filter signtool.exe -Recurse -ErrorAction SilentlyContinue |
    Sort-Object FullName -Descending
  $signTool = $candidates | Select-Object -First 1 -ExpandProperty FullName
}
if (-not $signTool) {
  throw "signtool.exe was not found on the Windows runner."
}

$tempPfx = Join-Path $env:RUNNER_TEMP "pc-manager-signing-$([guid]::NewGuid().ToString('N')).pfx"
try {
  [IO.File]::WriteAllBytes($tempPfx, [Convert]::FromBase64String($env:WINDOWS_SIGNING_PFX_BASE64))
  $timestampUrl = if ([string]::IsNullOrWhiteSpace($env:WINDOWS_SIGNING_TIMESTAMP_URL)) {
    "http://timestamp.digicert.com"
  } else {
    $env:WINDOWS_SIGNING_TIMESTAMP_URL
  }

  & $signTool sign /fd SHA256 /td SHA256 /tr $timestampUrl /f $tempPfx /p $env:WINDOWS_SIGNING_PFX_PASSWORD $ArtifactPath
  if ($LASTEXITCODE -ne 0) {
    throw "signtool failed with exit code $LASTEXITCODE."
  }

  & $signTool verify /pa /all $ArtifactPath
  if ($LASTEXITCODE -ne 0) {
    throw "Authenticode verification failed with exit code $LASTEXITCODE."
  }

  $signature = Get-AuthenticodeSignature -FilePath $ArtifactPath
  if ($signature.Status -ne "Valid") {
    throw "Authenticode status is $($signature.Status), expected Valid."
  }

  Write-Host "Authenticode signature verified for $(Split-Path $ArtifactPath -Leaf)."
}
finally {
  if (Test-Path -LiteralPath $tempPfx) {
    Remove-Item -LiteralPath $tempPfx -Force
  }
}
