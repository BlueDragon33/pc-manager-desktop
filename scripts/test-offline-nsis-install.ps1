# Disposable GitHub Actions Windows-runner test for the unsigned NSIS package.
# Validates that installation registers pcmanager:// for THIS Windows account
# and that uninstall removes only the handler installed by this package.
# Never run this against a non-disposable user profile.
param(
  [Parameter(Mandatory = $true)]
  [string]$InstallerPath
)
$ErrorActionPreference = 'Stop'
$schemeKey = 'HKCU:\Software\Classes\pcmanager'
$commandKey = Join-Path $schemeKey 'shell\open\command'
$exePath = $null
$installAttempted = $false

function Invoke-SilentInstaller([string]$file, [string]$action) {
  $process = Start-Process -FilePath $file -ArgumentList '/S' -Wait -PassThru
  if ($process.ExitCode -ne 0) { throw "$action returned exit code $($process.ExitCode)." }
}

if (-not (Test-Path -LiteralPath $InstallerPath -PathType Leaf)) {
  throw 'Unsigned preview installer was not produced.'
}
if (Test-Path -LiteralPath $schemeKey) {
  throw 'Disposable runner unexpectedly has an existing pcmanager:// handler. Refuse to overwrite.'
}

try {
  $installAttempted = $true
  Invoke-SilentInstaller -file $InstallerPath -action 'NSIS preview install'

  if (-not (Test-Path -LiteralPath $commandKey)) {
    throw 'NSIS install did not create the offline launch protocol.'
  }
  $stored = [string](Get-Item -LiteralPath $commandKey).GetValue('')
  if ($stored -notmatch '^"([^"]+\\PC Manager Desktop\.exe)" "%1"$') {
    throw ('Unexpected pcmanager handler after NSIS install: ' + $stored)
  }
  $exePath = $Matches[1]
  if (-not (Test-Path -LiteralPath $exePath -PathType Leaf)) {
    throw 'Registered PC Manager executable is missing after install.'
  }
  $uninstaller = Join-Path (Split-Path -Parent $exePath) 'uninstall.exe'
  if (-not (Test-Path -LiteralPath $uninstaller -PathType Leaf)) {
    throw 'NSIS uninstaller is missing.'
  }
  Invoke-SilentInstaller -file $uninstaller -action 'NSIS preview uninstall'
  if (Test-Path -LiteralPath $schemeKey) {
    throw 'Uninstaller left behind the pcmanager:// scheme it owned.'
  }
  Write-Output 'PASS unsigned NSIS install, per-user pcmanager:// ownership and uninstall cleanup.'
} finally {
  # A failed test must never delete a protocol it does not own.
  if ($installAttempted -and $exePath -and (Test-Path -LiteralPath $exePath)) {
    $uninstaller = Join-Path (Split-Path -Parent $exePath) 'uninstall.exe'
    if (Test-Path -LiteralPath $uninstaller -PathType Leaf) {
      try { Invoke-SilentInstaller -file $uninstaller -action 'Cleanup failed preview install' }
      catch { Write-Warning $_ }
    }
  }
}
