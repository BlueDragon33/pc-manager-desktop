# P8 EXECUTION PROMPT — App Manager Desktop Agent Gateway

Read `/AGENTS.md`, `/docs/MASTER_BUILD_PROMPT.md`, `/docs/IMPLEMENTATION_PLAN.md`, and `/docs/ARCHITECTURE.md` before changes.

## Approved architecture

P8 uses the user-approved **outbound-only Desktop Agent Gateway v1**.

PC Manager is a native Windows application behind normal NAT/firewall boundaries. Application Management must never require an inbound Windows port and must never call localhost on the managed PC.

Transport:

```text
PC Manager Windows
        |
        | outbound HTTPS
        v
Application Management
Desktop Agent Gateway
```

The paired server implementation lives in `BlueDragon33/Application-Management`.

## Identity

- application ID: `pc-manager`;
- platform: `windows`;
- device type: `desktop-native`;
- protocol: `pc-manager-agent/v1`;
- device key: persistent P-256 key in the Windows CNG user key store;
- private key must never be uploaded or written as plaintext by PC Manager;
- device registration is pending by default;
- server approval state is authoritative: pending / approved / blocked.

## Gateway lifecycle

1. Load or create the Windows CNG P-256 device identity.
2. Register the public JWK idempotently.
3. Read approval/license/release/update policy.
4. For approved devices, request a short-lived one-time challenge.
5. Sign:
   `pc-manager-agent/v1:<action>:<deviceId>:<challenge>`.
6. Send heartbeat.
7. Validate typed command envelopes.
8. Execute only the local allow-list mapping.
9. Obtain a fresh challenge and acknowledge each command.
10. Retry transport failures with bounded backoff.

## Remote command allow-list

Exactly:

- `CHECK_UPDATE`
- `RUN_HEALTH_SCAN`
- `REFRESH_DEVICE_STATUS`
- `DISABLE_LICENSE`

P8 behavior:

- CHECK_UPDATE is typed and acknowledged as unsupported until P10 provides the updater;
- RUN_HEALTH_SCAN invokes the existing local evidence-backed health path;
- REFRESH_DEVICE_STATUS performs no arbitrary OS action;
- DISABLE_LICENSE updates the entitlement state through the typed gateway flow.

Unknown command types must fail closed during deserialization.

## Explicit prohibitions

Do not add:

- arbitrary shell commands;
- arbitrary PowerShell supplied by server/frontend;
- arbitrary process execution;
- arbitrary registry mutation;
- arbitrary filesystem mutation;
- arbitrary URL download-and-run;
- a generic `command`, `script`, `executable`, `registryPath`, or `downloadUrl` field to the remote command envelope.

A fixed PC Manager-authored PowerShell helper used locally for Windows CNG interaction is allowed only because no remote/user script text enters it.

## Connection state

Expose:

- configured/unconfigured;
- online/offline;
- device ID/code;
- approval state;
- app version;
- release channel;
- entitlement/license state;
- update policy;
- last successful contact;
- retry/heartbeat interval;
- actionable status message.

Offline mode must not disable local PC management functionality.

## Endpoint configuration

- Production endpoint must be HTTPS.
- Loopback HTTP is allowed only for local development.
- Reject paths/query/fragment in the configured base origin.
- Endpoint configuration is not secret.
- Environment override `PC_MANAGER_APP_MANAGER_BASE_URL` may be used for development/deployment.
- UI Settings may persist the origin locally.

## Heartbeat and backoff

- normal heartbeat target: 60 seconds;
- retry sequence: 30, 60, 120, 240, capped at 300 seconds;
- no overlapping background/manual sync;
- preserve last known state while reporting offline transport failure.

## Acceptance

Implementation is ready for merge when:

- P-256 Windows CNG identity is implemented;
- registration/approval/heartbeat/policy state is typed;
- command enum is closed and tested against unknown types;
- background heartbeat and manual sync do not overlap;
- bounded backoff is tested;
- Settings shows gateway state without exposing private key material;
- frontend/Rust/native Windows CI are green;
- paired Application Management gateway CI is green.

Do not mark Production acceptance complete until the Application Management migration/deployment is explicitly approved and a real Windows device proves register → approve → heartbeat → typed command acknowledgement.
