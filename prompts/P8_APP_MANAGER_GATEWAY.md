# P8 EXECUTION PROMPT — App Manager Desktop Agent Gateway

Read `/AGENTS.md`, `/docs/MASTER_BUILD_PROMPT.md`, `/docs/IMPLEMENTATION_PLAN.md`, and `/docs/ARCHITECTURE.md` before changes.

## Goal

Connect native PC Manager to Application Management without exposing an inbound desktop port or creating any generic remote-execution primitive.

The approved P8 architecture is outbound-only:

```text
PC Manager Windows
        |
        | HTTPS initiated by PC Manager
        v
Application Management Desktop Agent Gateway
```

The server counterpart lives in `BlueDragon33/Application-Management` and uses protocol:

```text
application-management.desktop-agent/v1
```

## Identity and authentication

Stable application identity:

```json
{
  "appId": "pc-manager",
  "platform": "windows",
  "deviceType": "desktop-native"
}
```

Requirements:

- use a persistent Windows CNG P-256 key for device identity;
- never export or persist the private key as plaintext;
- register using the public JWK;
- authenticate heartbeat/result messages with a one-time challenge and ECDSA-SHA256 proof;
- challenge replay must fail;
- production gateway origin must use HTTPS;
- plain HTTP is allowed only for explicit loopback development.

## Runtime behavior

While PC Manager is open:

- register/refresh the device idempotently;
- heartbeat at the server-directed interval, bounded to conservative values;
- use exponential/bounded retry backoff when offline;
- surface pending/approved/blocked state honestly;
- surface entitlement and update policy honestly;
- do not block local maintenance functions during App Manager outage;
- do not upload filenames, browsing history, file contents, document contents, or arbitrary paths.

## Remote command allow-list

Only these commands are accepted:

- `CHECK_UPDATE`
- `RUN_HEALTH_SCAN`
- `REFRESH_DEVICE_STATUS`
- `DISABLE_LICENSE`

Unknown commands must fail deserialization/dispatch.

Explicitly prohibited:

- arbitrary shell;
- arbitrary PowerShell supplied by the server;
- arbitrary process execution;
- arbitrary registry mutation;
- arbitrary download-and-run;
- generic privileged file operations.

The Windows transport provider may be a fixed PC Manager-authored PowerShell/CNG/HTTPS provider, but server data must never become executable script source.

## Command semantics

### CHECK_UPDATE

P8 may report that the signed updater is not implemented until P9. Do not fake update availability.

### RUN_HEALTH_SCAN

Run the existing local Health Check and report only privacy-safe aggregates such as score, coverage and counts. Do not upload findings containing paths or personal data.

### REFRESH_DEVICE_STATUS

Return stable application/platform/device/version/phase metadata only.

### DISABLE_LICENSE

Apply/acknowledge the managed entitlement state. Do not damage local data or turn the command into arbitrary execution.

## UI

Settings/sidebar should expose:

- gateway configured/not configured;
- online/offline;
- device code;
- approval state;
- entitlement state;
- release channel;
- last connection error where useful.

Not configured/offline must be calm informational states, not scareware.

## Acceptance

P8 implementation is ready to merge when:

- server gateway contract exists and is independently CI-checked;
- client owns all App Manager protocol code in `app-manager-client`;
- persistent P-256 identity is used on Windows;
- registration and heartbeat are outbound-only;
- retry/backoff is bounded;
- typed commands are allow-listed by enum/dispatcher;
- no arbitrary execution surface exists;
- offline/unconfigured state leaves local PC Manager usable;
- frontend/Rust/native Windows CI are green;
- live Production handshake remains explicitly unverified until an approved gateway origin is configured and tested.
