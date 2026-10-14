# P15A — Windows Service protocol foundation (inactive)

Status: **contract/test foundation only**. This is not a registered Windows Service, a working IPC endpoint, or permission to elevate machine operations.

## Current safety boundary

- Rust crate: `services/windows-service`; pure read-only `lib.rs` functions.
- Wire schema: JSON with `protocol_version = 1`, bounded ASCII request ID (1–64 chars), and internally tagged `command.type`.
- Maximum frame size: 4,096 bytes; empty, oversized, malformed and version-incompatible frames fail closed.
- Exact initial commands: `GET_SERVICE_VERSION`, `GET_CAPABILITIES`.
- Capabilities explicitly report `privileged_operations_enabled = false`.
- Unexpected fields and any mutating, shell, PowerShell, startup, cleanup, service-control, arbitrary path or registry operation are rejected by deserialization.
- No IPC listener, service installer/registration, elevated token, network port, registry mutation or remote authority is introduced.
- `main.rs` remains the existing non-privileged skeleton.

## Non-negotiable gates before P15B can be implemented

1. Select a local-only Windows named-pipe or equivalent transport with restrictive security descriptors. Reject remote pipe access. Bound concurrent sessions, payload sizes, per-request deadlines and replay.
2. Identify and authorize the connecting Windows user/process from the **server-side security token**; a user-provided PID, username or claimed SID is not authentication. Do not rely on UI-only checks. Evaluate impersonation and session isolation.
3. Version both request and response contracts, detect mismatch and offer repair/update instructions. Never interpret unknown commands as a fallback action.
4. Implement service-side capability and ownership checks for each individual mutation using opaque local plan IDs; revalidate filesystem/registry targets and permissions just before performing an operation.
5. Persist a local operation audit before destructive work where possible; design safe rollback/compensation, failure isolation and idempotence. No arbitrary command execution.
6. Test unprivileged caller, cross-user caller, malicious request, oversized frame, timeout/disconnect, replay, concurrent requests and service crash on disposable Windows test fixtures.
7. Obtain human real-Windows acceptance before enabling any elevated mutation in a signed build.

## Local checks

Run `cargo test -p pc-manager-windows-service --locked`, `cargo fmt --check`, and full workspace CI. The P13 nine manual production gates remain independent and pending.
