# MASTER BUILD PROMPT — PC Manager Desktop

## Product objective

Build **PC Manager Desktop**, a native Windows 10/11 x64 system-management application.

The application should provide the genuinely useful capabilities users expect from tools such as CCleaner while improving on them in four areas:

1. Safety
2. Transparency
3. Reversibility
4. Centralized administration through App Manager

The app is not a "one-click magic optimizer". It is an evidence-based maintenance and monitoring tool.

---

## Product pillars

### A. Health

Give the user a concise, accurate answer to:

- Is the machine healthy?
- What is consuming storage?
- What starts with Windows?
- What can safely be cleaned?
- What is actually slowing the machine?
- Is hardware behaving normally?
- Are important updates available?

Health findings must be categorized into:

- Storage
- Performance
- Security
- Updates
- Privacy

A score may be shown, but every score must be traceable to concrete findings.

### B. Cleaning

Cleaning must distinguish:

- safe temporary files;
- browser cache;
- application cache;
- logs;
- recycle bin;
- user-selected large files;
- potentially risky files.

The scan phase and clean phase must be separate.

Never delete merely because a file is "old".

### C. Optimization

Optimization must focus on measurable causes:

- startup apps;
- unnecessary background processes;
- high-resource applications;
- scheduled tasks;
- services only when the impact is understood.

Do not claim that disabling arbitrary services "boosts performance".

### D. Monitoring

Provide useful live/near-live information for:

- CPU
- RAM
- disks
- network
- GPU where reliable
- storage health where reliable
- temperatures only when a trustworthy source is available

Do not fabricate unavailable sensor data.

### E. Recovery

Before significant changes, record enough state to explain and, when feasible, undo the change.

Restore Center should eventually show:

- action time;
- feature;
- files/settings affected;
- expected effect;
- rollback availability;
- rollback result.

---

## Initial UI

Desktop shell concept:

```text
┌─────────────────────────────────────────────────────────────┐
│ PC Manager                                             _ □ X│
├──────────────┬──────────────────────────────────────────────┤
│ Overview     │                                              │
│ Health Check │              Main content                    │
│ Smart Clean  │                                              │
│ Startup      │                                              │
│ Apps         │                                              │
│ Storage      │                                              │
│ Duplicates   │                                              │
│ Monitor      │                                              │
│ Restore      │                                              │
│              │                                              │
│ Settings     │                                              │
└──────────────┴──────────────────────────────────────────────┘
```

Overview should answer the user's most important questions without forcing navigation.

Example:

```text
PC HEALTH 92/100

Storage       96  OK
Performance   84  Attention
Security     100  OK
Updates       88  Attention
Privacy       91  OK

8.4 GB can be safely reclaimed
6 startup apps have measurable boot impact
3 software updates are recommended
0 serious security findings
```

Use restrained language.

---

## Technical architecture

### Desktop UI

React + TypeScript + Vite.

Responsibilities:

- rendering;
- navigation;
- local UI state;
- accessibility;
- calling typed Tauri commands;
- presenting evidence and confirmation dialogs.

The UI must never contain the authoritative safety decision for privileged actions.

### Tauri layer

Responsibilities:

- native application lifecycle;
- secure command bridge;
- window management;
- updater integration;
- communication with Rust core.

### pc-core

Platform-neutral domain logic where possible:

- scan models;
- findings;
- risk classifications;
- cleanup plans;
- rollback records;
- scoring;
- feature orchestration.

### pc-windows

Windows-specific implementation:

- known folders;
- registry reads/writes for defined features;
- startup sources;
- services;
- processes;
- installed applications;
- storage inventory;
- Windows Update information where supported.

Prefer documented Windows APIs.

### Windows Service

Only operations requiring elevation should cross to the privileged service.

Use a narrow IPC interface such as:

```text
GetServiceVersion
ExecuteCleanupPlan(plan_id)
SetStartupEntry(entry_id, enabled)
UninstallApplication(app_id, confirmation_token)
RestoreOperation(operation_id)
```

The service must not expose a generic "execute command" method.

### Local SQLite

Suggested logical tables:

- device_state
- scan_runs
- scan_findings
- cleanup_plans
- cleanup_items
- operations
- rollback_records
- settings
- app_manager_state

Do not store unnecessary sensitive content.

---

## App Manager contract

Desktop identity:

```json
{
  "appId": "pc-manager",
  "platform": "windows",
  "deviceType": "desktop-native"
}
```

Suggested client lifecycle:

```text
launch
  ↓
load local device identity
  ↓
register/refresh device
  ↓
check approval/license
  ↓
send heartbeat
  ↓
check release policy
  ↓
run normal application
```

The desktop app should tolerate temporary loss of network connectivity.

App Manager outages must not corrupt local maintenance functions.

The client must not send:

- personal file content;
- browsing history;
- document contents;
- arbitrary local file paths unless explicitly required and privacy-reviewed.

---

## Update model

Release channels:

- dev
- beta
- stable

Update metadata should include:

```json
{
  "version": "1.2.0",
  "channel": "stable",
  "mandatory": false,
  "sha256": "...",
  "releaseNotes": [],
  "publishedAt": "..."
}
```

Before installing:

- verify signature/update metadata;
- verify checksum;
- ensure package matches platform/architecture.

---

## Feature specifications

### Health Check

Inputs:

- storage state;
- safe-clean candidates;
- startup impact;
- basic update state;
- system resource pressure;
- obvious system configuration problems.

Outputs:

- grouped findings;
- evidence;
- risk;
- recommendation;
- action availability.

Do not automatically execute changes after scanning.

### Smart Cleaner

V1 supports only high-confidence locations.

Each candidate needs:

- category;
- path/provider ID;
- bytes;
- confidence;
- last modified when relevant;
- locked/in-use state;
- safe-to-delete classification.

Use a cleanup plan that is generated first and executed second.

### Startup Manager

Enumerate, where practical:

- startup registry entries;
- startup folders;
- supported scheduled startup tasks.

Each item should expose:

- publisher if known;
- source;
- command;
- enabled state;
- impact estimate only when evidence exists.

### Uninstaller

Enumerate installed software from trustworthy Windows sources.

Support:

- launch standard uninstaller;
- refresh inventory;
- identify stale records cautiously.

Do not silently remove applications.

### Duplicate Finder

Use a staged approach:

1. group by size;
2. partial hash for candidates;
3. full hash before declaring identical.

Do not delete duplicates automatically.

### Storage Analyzer

Provide:

- category summaries;
- largest folders/files;
- extension/type summaries where useful;
- exclusions for system-sensitive areas.

### System Monitor

Show live data without pretending to replace specialist diagnostic software.

Keep sampling overhead low.

### Restore Center

Every supported reversible operation produces an operation record.

Rollback must be idempotent where possible.

---

## Phase gates

No phase is considered complete just because UI exists.

Each phase requires:

- correct data flow;
- error handling;
- tests;
- CI;
- documentation;
- a usable demo path.

Do not move to destructive features before the recovery model exists.

---

## Quality targets

- App launches quickly on typical Windows 10/11 hardware.
- Idle resource usage should remain low.
- Background monitoring must be configurable.
- A failed feature should not crash the whole app.
- Every privileged action must be logged locally.
- No unhandled promise rejections/panics in normal paths.
- No hard-coded developer machine paths.
- No secrets committed.
- No production release when CI is red.

---

## Build philosophy

Work phase by phase.

Do not generate hundreds of files only to create the appearance of progress.

Prefer:

- small verified increments;
- real Windows behavior;
- typed interfaces;
- tests around risky code;
- explicit TODOs for unimplemented OS cases.

At the end of each phase, produce a concise evidence report:

```text
Phase:
Completed:
Tests:
Build:
Known limitations:
Next phase:
```

The implementation plan is defined in `docs/IMPLEMENTATION_PLAN.md`.
