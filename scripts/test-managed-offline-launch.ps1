# Disposable Windows-runner contract test; no PC Manager installation required.
$ErrorActionPreference = 'Stop'
$helper = Join-Path $PSScriptRoot 'register-managed-offline-launch.ps1'
$root = Join-Path $env:TEMP ("pc-manager-launch-" + [guid]::NewGuid().ToString('N'))
$first = Join-Path $root 'a\pc-manager-desktop.exe'
$second = Join-Path $root 'b\pc-manager-desktop.exe'
$reg = 'HKCU:\Software\Classes\pcmanager'
try {
    New-Item (Split-Path $first) -ItemType Directory -Force | Out-Null
    New-Item (Split-Path $second) -ItemType Directory -Force | Out-Null
    Set-Content -LiteralPath $first -Value 'disposable fixture' -NoNewline
    Set-Content -LiteralPath $second -Value 'disposable fixture' -NoNewline

    # Never delete an existing user protocol in a developer's Windows account.
    if (Test-Path -LiteralPath $reg) {
        throw 'Protocol pcmanager:// already registered. Refusing to mutate existing key.'
    }

    & $helper -ExecutablePath $first
    $actual = (Get-Item -LiteralPath (Join-Path $reg 'shell\open\command')).GetValue('')
    $expected = '"{0}" "%1"' -f ([System.IO.Path]::GetFullPath($first))
    if ($actual -ne $expected) {
        Write-Output ('Expected command: ' + $expected)
        Write-Output ('Stored command: ' + $actual)
        throw 'Protocol command did not round-trip.'
    }

    & $helper -ExecutablePath $first
    $actual = (Get-Item -LiteralPath (Join-Path $reg 'shell\open\command')).GetValue('')
    if ($actual -ne $expected) { throw 'Idempotent registration was not preserved.' }

    $rejected = $false
    try { & $helper -ExecutablePath $second } catch { $rejected = $true }
    if (-not $rejected) { throw 'Competing handler should have been rejected.' }
    $actual = (Get-Item -LiteralPath (Join-Path $reg 'shell\open\command')).GetValue('')
    if ($actual -ne $expected) { throw 'Registration was overwritten by competitor.' }

    & $helper -ExecutablePath $first -Unregister
    if (Test-Path -LiteralPath $reg) { throw 'Unregister left protocol key behind.' }

    Write-Output 'PASS pcmanager:// current-user register/idempotence/conflict/unregister'
} finally {
    # In case of test failure, only undo a registration pointing at our fixture.
    if (Test-Path -LiteralPath (Join-Path $reg 'shell\open\command')) {
        $owned = (Get-Item -LiteralPath (Join-Path $reg 'shell\open\command')).GetValue('')
        if ($owned -eq ('"{0}" "%1"' -f ([System.IO.Path]::GetFullPath($first)))) {
            Remove-Item -LiteralPath $reg -Recurse -Force
        }
    }
    if (Test-Path -LiteralPath $root) { Remove-Item -LiteralPath $root -Recurse -Force }
}
