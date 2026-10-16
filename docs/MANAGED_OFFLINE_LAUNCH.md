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

1. Install PC Manager Desktop using its Windows NSIS installer (signed Stable
   only after acceptance; unsigned Preview for disposable Windows testing).
   The current-user installer registers `pcmanager://` automatically in HKCU.
2. Open Application Management on the **same Windows user account** and click
   the PC Manager icon or **Mở PC Manager cục bộ**.
3. Approve the browser's external-application prompt, if displayed. Windows
   opens the packaged offline React/Tauri interface without a localhost server.

The manual registration script is a recovery/portable-build fallback, not a
normal post-install requirement. It accepts only the installed
`pc-manager-desktop.exe` executable and must be run explicitly.

The Windows protocol registration deliberately **does not forward the browser
URL as a command-line argument**. It starts the installed executable with no
URL parameters, so an external page cannot inject arguments or deep-link
commands into the native application. The app opens its standard interface.
The integration is a launch-only pilot, not a verified installed-app handshake.

To unregister the URL scheme for the same executable:
`./scripts/register-managed-offline-launch.ps1 -ExecutablePath '<absolute installed path>/pc-manager-desktop.exe' -Unregister`

The registration helper refuses to replace another handler **or an existing
handler-less protocol key**. It writes only
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
- Repeat clicks on Windows use a local session mutex and activation event:
  the second process exits after signalling the first process to show,
  unminimize and focus its window. No browser URL or command is forwarded.
  Windows can still restrict foreground focus.
- Signed installation, production upgrades and the actual Chrome/Edge
  handoff on the user's Windows machine remain separate unverified gates.
  Mutex/event unit tests are not real browser or production acceptance.
- The installer registers only a missing scheme or one owned by the same
  executable path; legacy `"%1"` entries for that exact executable are
  migrated to the argument-free handler. Any existing foreign or incomplete
  scheme is preserved. Uninstall removes only its own exact handler.
- NSIS `installMode` is `currentUser`; do not silently switch to HKLM.
- On Linux, macOS and mobile devices, PC Manager's Windows runtime is
  unavailable. Do not substitute an online or remote system-admin URL.
- The nine real-Windows acceptance gates in `docs/release-acceptance.json`
  remain pending; this pilot does not grant Production release authority.

## CI proof

`scripts/test-managed-offline-launch.ps1` tests manual registration,
ownership conflicts, legacy migration and unregister on a disposable Windows
runner. Preview CI also runs `scripts/test-offline-nsis-install.ps1` against
an actual unsigned NSIS package, checking the executable, argument-free
handler and owned uninstall. This is not signed Production acceptance or
proof of real browser-to-installed-application handoff.
