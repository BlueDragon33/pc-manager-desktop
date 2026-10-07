# ARCHITECTURE

## Overview

PC Manager Desktop is intentionally split into trust boundaries.

```text
┌─────────────────────────────────────────┐
│ React + TypeScript UI                   │
│ unprivileged                            │
└────────────────┬────────────────────────┘
                 │ typed Tauri commands
┌────────────────▼────────────────────────┐
│ Tauri / application host               │
│ unprivileged by default                 │
└──────────────┬──────────────┬───────────┘
               │              │
       ┌───────▼───────┐  ┌──▼──────────────────────┐
       │ pc-core       │  │ app-manager-client      │
       │ domain logic  │  │ registration/heartbeat  │
       └───────┬───────┘  └─────────────────────────┘
               │
       ┌───────▼────────┐
       │ pc-windows     │
       │ Windows reads  │
       └───────┬────────┘
               │ only when elevation is required
       ┌───────▼───────────────────────────┐
       │ PC Manager Windows Service        │
       │ privileged, narrow allow-list IPC │
       └───────────────────────────────────┘
```

## Trust boundaries

### UI boundary

The UI is not trusted to authorize privileged actions.

It may request:

```text
disable startup item X
```

but the service/core must verify that X is a known startup item and that the requested operation is valid.

### Privileged service boundary

The Windows Service exposes only high-level domain operations.

Good:

```text
SetStartupEntry(entry_id, false)
ExecuteCleanupPlan(plan_id)
RestoreOperation(operation_id)
```

Bad:

```text
RunCommand(string)
RunPowerShell(string)
WriteRegistry(path, value)
DeleteFile(path)
```

Generic privileged primitives are prohibited because they would make remote compromise dramatically more dangerous.

## Crates

### pc-core

Owns domain concepts:

- HealthFinding
- ScanRun
- CleanupCandidate
- CleanupPlan
- OperationRecord
- RollbackRecord
- RiskLevel
- Recommendation

No UI dependencies.

### pc-windows

Owns Windows-specific data acquisition and operations that do not need a persistent privileged service.

Examples:

- system inventory;
- known folders;
- installed apps;
- startup enumeration;
- process metrics.

### pc-monitor

Owns sampling and resource-monitoring abstractions.

It must rate-limit and avoid high polling overhead.

### pc-updater

Owns:

- update metadata parsing;
- channel policy;
- checksum verification;
- handoff to signed updater/install mechanism.

### app-manager-client

Owns all App Manager protocol code.

No other module should directly call App Manager endpoints.

Responsibilities:

- registration;
- authentication/session state;
- heartbeat;
- entitlement;
- release policy;
- typed remote commands.

## Windows Service

The service should be separately versioned at protocol level even if shipped from the same repository.

Example:

```text
serviceProtocolVersion = 1
```

The desktop app should reject incompatible privileged protocol versions with a clear repair/update message.

## IPC design principles

- local machine only;
- authenticated/authorized local caller where practical;
- strongly typed messages;
- size limits;
- versioned schema;
- timeouts;
- explicit error codes;
- no user-provided command lines crossing directly into elevated execution.

## Local persistence

SQLite is the preferred local store.

The DB is application state, not a dumping ground for system data.

Suggested tables:

```text
schema_version
settings
device_state
scan_runs
scan_findings
cleanup_plans
cleanup_items
operations
rollback_records
app_manager_state
```

Large transient scan data should be bounded and pruned.

## Contracts

The `contracts/` directory should eventually contain schemas for interfaces shared across components, for example:

- device registration;
- heartbeat;
- update metadata;
- remote command envelope;
- service IPC protocol.

Schemas should include explicit version fields.

## Error model

User-facing layers receive structured errors, for example:

```text
code
message
recoverable
requires_elevation
source
details_for_log
```

Never surface raw panic traces as the primary user message.

## Logging

Logs should support diagnosis without collecting sensitive contents.

Good:

```text
cleanup provider chrome-cache: 124 files, 382 MB, 3 locked
```

Avoid:

```text
uploaded full browsing path history and filenames
```

## Future portability

Windows is the only initial target.

Do not prematurely build Linux/macOS support, but keep core domain models free from unnecessary Windows types so future ports remain possible.


## Operational sovereignty

PC Manager Desktop is a `LOCAL_CORE` application.

The canonical authority chain for machine mutations is local:

`UI request → Tauri/Rust domain validation → narrow privileged Windows Service IPC → local Windows state`

Remote services are replaceable adapters:

- App Manager: optional identity/lifecycle/update coordination;
- Google Drive or equivalent: optional encrypted settings/report backup;
- future AI providers: optional advisory analysis only.

Loss of Internet or any remote provider must not disable ordinary local health scans, cleaning-plan review/execution, startup management, monitoring or restore operations.

No cloud service may own arbitrary privileged execution, cleanup decisions, rollback authority, private keys or raw sensitive machine inventories.

Canonical dependency posture: `.blueprint/dependency-budget.json`.
