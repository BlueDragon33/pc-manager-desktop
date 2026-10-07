param(
  [Parameter(Mandatory = $false)]
  [string]$OutputDirectory = "artifacts"
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($env:WINDOWS_CODE_SIGNING_PFX_BASE64) -or [string]::IsNullOrWhiteSpace($env:WINDOWS_CODE_SIGNING_PFX_PASSWORD)) {
  throw "WINDOWS_CODE_SIGNING_PFX_BASE64 and WINDOWS_CODE_SIGNING_PFX_PASSWORD are required."
}

New-Item -ItemType Directory -Force $OutputDirectory | Out-Null
$temp = Join-Path $env:RUNNER_TEMP "pc-manager-signing"
New-Item -ItemType Directory -Force $temp | Out-Null
$pfxPath = Join-Path $temp "codesign.pfx"

try {
  [IO.File]::WriteAllBytes($pfxPath, [Convert]::FromBase64String($env:WINDOWS_CODE_SIGNING_PFX_BASE64))
  $password = ConvertTo-SecureString $env:WINDOWS_CODE_SIGNING_PFX_PASSWORD -AsPlainText -Force
  $certificate = Import-PfxCertificate -FilePath $pfxPath -CertStoreLocation Cert:\CurrentUser\My -Password $password
  if (-not $certificate -or [string]::IsNullOrWhiteSpace($certificate.Thumbprint)) { throw "Unable to import the Windows code-signing certificate." }

  $overlayPath = Join-Path $temp "tauri.release.conf.json"
  @{
    bundle = @{
      active = $true
      targets = @("nsis")
      windows = @{
        certificateThumbprint = $certificate.Thumbprint
        digestAlgorithm = "sha256"
        timestampUrl = "http://timestamp.digicert.com"
      }
    }
  } | ConvertTo-Json -Depth 8 | Set-Content $overlayPath

  Push-Location apps/desktop
  try {
    npx tauri build --bundles nsis --config $overlayPath
    if ($LASTEXITCODE -ne 0) { throw "Tauri signed bundle build failed." }
  } finally {
    Pop-Location
  }

  $installers = Get-ChildItem target/release/bundle/nsis/*.exe
  if (-not $installers) { throw "No NSIS installer was produced." }
  $installers | Copy-Item -Destination $OutputDirectory

  foreach ($installer in Get-ChildItem $OutputDirectory/*.exe) {
    $signature = Get-AuthenticodeSignature $installer.FullName
    if ($signature.Status -ne "Valid") { throw "Installer signature is not valid: $($installer.Name) ($($signature.Status))" }
  }
} finally {
  Remove-Item $pfxPath -Force -ErrorAction SilentlyContinue
}
