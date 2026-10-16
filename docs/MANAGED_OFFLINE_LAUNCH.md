# PC Manager — Offline Web App Launch from Application Management

## Runtime and ownership

PC Manager **already uses an offline web interface**: its React/Vite bundle is
packaged in the Tauri/Windows WebView2 desktop executable and calls local Rust
commands using the Tauri bridge. Its UI is not an HTTPS website and does not
require a localhost HTTP server.

Application Management remains the **control plane** for device approval,
heartbeat, release channel and typed non-destructive coordination, but does not
own files, registry changes or cleanup execution on the PC.

## Opening PC Manager from the management icon

The only approved local launch action is `pcmanager://open`:

1. Install PC Manager Desktop from its Windows NSIS installer (signed stable
   only after separate acceptance; unsigned Preview is for testing only).
   The installer automatically registers `pcmanager://` for the **current
   Windows account** (HKCU), without PowerShell or an inbound HTTP port.
2. Open Application Management on the **same Windows account/device** and
   click the **PC Manager** app icon or the **Mở PC Manager cục bộ** link.
3. Confirm the browser's external-application handoff if prompted. Windows
   opens the packaged offline React/Tauri application.

If a foreign handler already owns `pcmanager://`, installation leaves it
untouched and writes a diagnostic line instead of overwriting it. Uninstall
removes the registry association **only if** its command still matches the
installed PC Manager executable.

The manual per-user script remains available for development, portable builds
and explicit repair; it is no longer required after a normal NSIS install:
`./scripts/register-managed-offline-launch.ps1 -ExecutablePath '<absolute installed path>/PC Manager Desktop.exe'`.

The installed application currently ignores command-line URL parameters. It
opens its standard interface and does **not** interpret URLs as commands.
The integration is a launch-only pilot, not a verified installed-app handshake.

To unregister the URL scheme for the same executable:
`./scripts/register-managed-offline-launch.ps1 -ExecutablePath '<absolute installed path>/PC Manager Desktop.exe' -Unregister`

The registration helper refuses to replace another handler. It writes only
`HKCU:\Software\Classes\pcmanager` after explicit local invocation, never
`HKLM`, and is not an endpoint for App Manager remote commands.

## Limitations and acceptance gates

- No privilege escalation, localhost TCP listener, browser-to-Rust proxy,
  browser-minted device identity or filesystem URL parameters.
- Opening the URI from a browser **does not prove that an executable is
  installed, signed, running or approved**. A Windows/browser prompt or
  missing-handler error may appear. Status remains based on authenticated
  Desktop Agent evidence, not the launcher.
- A normal external Chrome/Edge tab cannot call the Tauri `invoke` bridge.
  Rehosting the frontend on `http://127.0.0.1` would disable native operations
  unless a separately designed and authenticated local service exists, which
  this pilot explicitly does not create.
- Each browser click may spawn a new app instance. Single-instance handling,
  a **signed** installer and end-to-end Windows browser handoff/install/upgrade
  tests remain separate release gates. NSIS lifecycle hooks provide only
  per-user registration and conservative ownership-aware removal.
- Current installer mode is explicitly `currentUser`. Do not silently change
  to `perMachine` or `both`: this hook does not write HKLM and must not be
  elevated into machine-wide protocol ownership.
- On Linux, macOS and mobile devices, PC Manager's Windows runtime is
  unavailable. Do not substitute an online or remote system-admin URL.
- The nine real-Windows acceptance gates in `docs/release-acceptance.json`
  remain pending; this pilot does not grant Production release authority.

## CI proof

`scripts/test-managed-offline-launch.ps1` tests the optional manual
registration helper, idempotence, non-overwrite of a different owner, and
owned unregister. `scripts/test-offline-nsis-install.ps1` is run from the
unsigned Preview workflow on a disposable Windows runner and validates an
actual NSIS install, current-user association, installed executable, and
uninstall cleanup. Browser permission prompts and production signing/upgrade
still need separate real-Windows acceptance. No P13 release gate is waived.
