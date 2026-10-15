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

1. Install PC Manager Desktop on a Windows machine (signed stable release
   after acceptance, or explicitly identified unsigned preview for testing).
2. Explicitly register a **per-user** protocol on that machine using:
   `./scripts/register-managed-offline-launch.ps1 -ExecutablePath '<absolute installed path>/PC Manager Desktop.exe'`
3. Open Application Management and click the **PC Manager** app icon or
   the **Mở PC Manager cục bộ** link in its management workspace.
4. The browser asks Windows to open the installed local Tauri app. It does not
   navigate to a public website or expose a local HTTP API.

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
  installer-time protocol registration/unregistration, a signed installer and
  an end-to-end launch/install/upgrade test remain separate release gates.
- On Linux, macOS and mobile devices, PC Manager's Windows runtime is
  unavailable. Do not substitute an online or remote system-admin URL.
- The nine real-Windows acceptance gates in `docs/release-acceptance.json`
  remain pending; this pilot does not grant Production release authority.

## CI proof

`scripts/test-managed-offline-launch.ps1` tests registration, idempotence,
non-overwrite of an existing different owner, and owned unregister using a
disposable Windows runner. Browser protocol handoff still needs manual human
verification after an actual Windows install.
