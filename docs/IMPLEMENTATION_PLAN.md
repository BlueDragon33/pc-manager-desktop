# IMPLEMENTATION PLAN

This is the execution roadmap for PC Manager Desktop.

Legend:

- [ ] not started
- [~] in progress
- [x] complete only after acceptance criteria are proven

---

## P0 — Repository foundation

Goal: create a reproducible monorepo before feature development.

Tasks:

- [x] Rust workspace
- [x] React + TypeScript + Vite desktop UI
- [x] Tauri desktop shell
- [x] workspace-level formatting/lint commands
- [x] basic unit-test commands
- [x] `.editorconfig`
- [x] sensible `.gitignore`
- [x] GitHub Actions CI for Rust + TypeScript
- [x] architecture docs wired into README
- [x] initial app metadata: `appId=pc-manager`

Acceptance:

- clean clone can install/build;
- UI opens as a native desktop app;
- CI passes on main;
- no feature logic yet beyond a shell.

---

## P1 — Application shell and design system

Goal: create the durable UI frame.

Tasks:

- [x] title bar/window strategy — retain native Tauri/Windows chrome for the initial release
- [x] sidebar navigation
- [x] Overview
- [x] Health Check
- [x] Smart Clean
- [x] Startup
- [x] Apps
- [x] Storage
- [x] Duplicates
- [x] Monitor
- [x] Restore
- [x] Settings
- [x] dark theme
- [x] light theme
- [x] keyboard/focus behavior
- [x] responsive support down to 1366×768
- [x] shared card/badge states; destructive dialogs and progress flows are completed in the first phases that need them

Acceptance:

- navigation is functional;
- no copyrighted CCleaner assets are copied;
- no fake system numbers are presented as real data;
- Windows native build and CI are green;
- user approved the P1 visual direction before P2 began.

---

## P2 — System inventory foundation

Goal: establish trustworthy read-only Windows data.

Implement typed models for:

- [x] OS/version
- [x] hostname/device ID abstraction
- [x] CPU summary
- [x] RAM summary
- [x] disk volumes
- [x] free/used storage
- [x] processes summary
- [x] installed applications
- [x] startup sources
- [x] basic network adapters

Architecture:

```text
UI
 ↓
Tauri commands
 ↓
pc-core interfaces
 ↓
pc-windows implementations
```

Acceptance:

- read-only;
- handles permission/access failures;
- unit tests for parsing/mapping logic;
- no Administrator requirement for normal inventory;
- Windows CI executes the real inventory provider and verifies hostname, OS, memory, CPU, and local-volume data;
- frontend, Rust, and Windows native build gates are green.

---

## P3 — Health Check V1

Goal: produce an explainable health report.

Categories:

- [x] Storage
- [x] Performance
- [x] Security — explicit unavailable state in V1 until an evidence-backed check is implemented
- [x] Updates — explicit unavailable state in V1 until trusted update checks are implemented
- [x] Privacy — explicit unavailable state in V1 until an evidence-backed check is implemented

V1 may have limited Security/Updates/Privacy coverage, but unsupported checks must be shown as unavailable rather than guessed.

Each finding:

```text
id
category
title
description
evidence
severity
risk
recommended_action
action_available
reversible
```

Acceptance:

- Scan Now performs real checks from a fresh P2 Windows inventory;
- progress can be cancelled safely by invalidating the UI request and discarding any late native result;
- the latest completed result is persisted locally; cancelled and failed scans are not persisted;
- score derives only from supported, evidence-backed findings and reports partial coverage honestly;
- unsupported categories remain explicit `unavailable` states and do not reduce the score;
- no destructive action or Administrator privilege is used during scan;
- frontend, Rust, Windows inventory smoke test, and native Windows build are green in CI.

---

## P4 — Smart Cleaner + Restore Center foundation

Goal: first write/destructive subsystem, built safely.

Smart Cleaner V1:

- [x] Windows temp candidates
- [x] application temp providers with explicit rules
- [x] browser cache providers with explicit rules
- [~] recycle bin as a separate opt-in item — modeled and opt-in, native size/count provider still gated
- [x] scan-only preview
- [x] cleanup plan
- [x] execution result

Restore foundation:

- [x] operation log — append-only local JSONL audit under LOCALAPPDATA
- [x] before-state metadata — native plan ID, requested bytes/files, provider totals, and per-item scan metadata used for execution revalidation; personal paths are not persisted in history
- [x] rollback capability flags — cache/temp deletion is explicitly marked not restorable
- [x] rollback UI — Restore Center shows operation history and honest rollback availability

Important:

Not every deleted cache file is restorable. The app must be honest about that. "Restore Center" records operations and restores only operations for which a safe rollback exists.

Acceptance:

P4A preview gate:
- [x] exact byte estimate before cleanup;
- [x] user can inspect categories/providers;
- [x] no arbitrary filesystem root can be supplied by the frontend;
- [x] reparse points/symlinks are skipped;
- [x] provider errors are isolated into warnings;
- [x] native cleanup plan is retained by plan ID;
- [x] no delete command exists in P4A;
- [x] Windows CI executes the real built-in scanner and native build successfully;
- [x] user verified preview totals and provider breakdown on a real Windows machine before P4B.

P4B execution gate:
- [x] every planned path is canonicalized and revalidated immediately before deletion;
- [x] reparse points, changed files, out-of-root paths, unsupported providers, and stale-rule failures are skipped;
- [x] errors do not abort unrelated cleanup items;
- [x] audit storage is preflighted before deletion and the completed operation record is appended locally;
- [x] execution result shows requested/deleted/failed files and bytes accurately;
- [x] restore capability is labeled honestly as "Not restorable" for cache/temp deletion;
- [x] disposable Windows CI tests prove deletion of an unchanged planned file and rejection of a candidate changed after scan;
- [x] frontend, Rust, Clippy, Windows tests, and native Windows build are green.

---

## P5 — Startup Manager

Goal: safe startup control.

Sources:

- [x] supported Run/RunOnce registry locations
- [x] startup folders
- [x] selected scheduled tasks when they clearly represent startup behavior

Capabilities:

- [x] inventory
- [x] enable/disable where safe
- [x] publisher/source display — publisher remains unavailable when Windows does not provide trustworthy evidence
- [x] command display
- [x] rollback record
- [x] search/filter

Acceptance:

- [x] supported current-user Run/RunOnce entries preserve rollback state in the PC Manager backup namespace;
- [x] current-user Startup folder items use a PC Manager-owned disabled store and restore to their recorded Startup folder;
- [x] machine-wide/elevated entries remain read-only instead of elevating the whole desktop app;
- [x] scheduled startup/logon tasks are discovered; unsupported permission changes fail closed;
- [x] disabling startup never invokes an uninstaller;
- [x] every attempted mutable change produces an operation record;
- [x] frontend, Rust tests, Clippy, and native Windows build pass in CI;
- [~] real Windows verification: inventory has been proven on the user's Windows machine; the disposable enable-disable-enable round trip is intentionally deferred by user direction and remains an open P5 verification item.

Sequencing exception recorded 2026-10-06: P6 implementation may proceed while this P5 round-trip remains open. P5 must not be represented as fully accepted until that round-trip is completed.

---

## P6 — Apps, Duplicates, Storage

### Uninstaller

- [x] enumerate installed programs
- [x] search/filter
- [x] open standard uninstall flow
- [x] refresh after uninstall — explicit refresh after the external uninstaller returns
- [x] no forced silent uninstall in V1

### Duplicate Finder

- [x] folder selection — native Windows folder picker with session-scoped root IDs
- [x] exclusions
- [x] size grouping
- [x] partial hashing
- [x] full hashing
- [x] preview
- [x] user-selected deletion only

### Storage Analyzer

- [x] tree aggregation
- [x] largest files
- [x] file-type grouping
- [x] exclusions
- [x] cancellation

Acceptance:

- [x] symlink/reparse-point loops are skipped by the shared filesystem walker;
- [x] inaccessible paths become bounded warnings instead of crashing scans;
- [x] duplicates are declared only after exact-size, partial-hash, and full-hash verification;
- [x] duplicate deletion uses opaque scan IDs, preserves at least one verified copy, revalidates path/metadata/full hash immediately before deletion, and records an audit result;
- [x] Apps rediscover the selected opaque app ID before launch and never accept a frontend-supplied command line;
- [x] silent uninstall flags are rejected and MSI uninstall uses the standard interactive flow;
- [x] frontend lint/typecheck/tests/build/format, Rust fmt/check/tests/Clippy, Windows provider tests, and native Windows build are green in CI;
- [ ] representative P6 behavior is verified manually on a real Windows machine — deferred by explicit user instruction.

---

## P7 — System Monitor

Goal: low-overhead useful telemetry.

- [x] CPU
- [x] RAM
- [x] disk activity where reliable
- [x] network
- [x] process top consumers
- [x] GPU where reliable
- [x] sensor capability detection

Acceptance:

- [x] unsupported sensors show "Unavailable" instead of fabricated values;
- [x] sampling interval is configurable with conservative 2s/5s/10s choices;
- [x] monitor can be paused and requests no new native samples while paused;
- [x] every native snapshot records provider sample duration and the UI surfaces it;
- [x] Windows CI executes the real monitor provider and native build successfully;
- [x] frontend lint/typecheck/tests/build/format and Rust fmt/check/tests/Clippy are green before merge;
- [ ] representative idle-overhead behavior is measured manually on a real Windows machine — deferred by explicit user instruction.

---

## P8 — App Manager integration

Goal: connect PC Manager to the existing administration platform through the approved outbound-only Desktop Agent Gateway.

Client capabilities:

- [x] device identity — persistent Windows CNG P-256 key; only the public JWK leaves the device
- [x] registration — idempotent `pc-manager/windows/desktop-native` registration
- [x] approval state — pending/approved/blocked
- [x] heartbeat — one-time signed challenge with server-directed interval
- [x] online/offline semantics — heartbeat presence is separate from local PC Manager availability
- [x] app version
- [x] release channel
- [x] entitlement/license state
- [x] update policy
- [x] typed remote command envelope

Remote command allow-list initially:

- [x] CHECK_UPDATE
- [x] RUN_HEALTH_SCAN
- [x] REFRESH_DEVICE_STATUS
- [x] DISABLE_LICENSE

Explicitly prohibited:

- arbitrary shell;
- arbitrary PowerShell supplied by the server;
- arbitrary process execution;
- arbitrary registry mutation;
- arbitrary file download-and-run.

Acceptance:

- [x] offline/not-configured mode leaves local PC Manager maintenance usable;
- [x] retries use bounded backoff;
- [x] the device private key remains in Windows CNG instead of plaintext application storage;
- [x] server commands deserialize into and dispatch through a four-command allow-list only;
- [x] health-scan command returns privacy-bounded aggregate evidence instead of paths/file contents;
- [x] Application Management exposes a separate authenticated admin surface while the native transport is outbound-only;
- [x] frontend/Rust/native Windows CI are green for the final P8 client head;
- [ ] Production gateway migration/deployment and a real signed PC Manager handshake are verified — deferred until the approved HTTPS Application Management origin is deployed/configured.

---

## P9 — Production packaging and update system

Goal: reliable release pipeline.

- [x] preview build workflow — unsigned NSIS artifact, explicitly labeled preview-only
- [x] production workflow — manual signed release from `main`
- [x] production confirmation input — exact `DEPLOY_PRODUCTION`
- [x] Windows installer — NSIS bundle target
- [x] checksums — SHA-256 manifest plus per-artifact metadata digest
- [x] signing integration hooks — production fails closed without the configured PFX credentials
- [x] release notes — generated from the release commit history
- [x] updater metadata — `pc-manager.update/v1`
- [x] stable/beta/dev policy — enforced by `pc-updater` policy helpers
- [x] rollback/failed-update strategy — documented in `docs/RELEASE_POLICY.md`

Desired production trigger pattern:

```powershell
gh workflow run production-release.yml --ref main -f confirm=DEPLOY_PRODUCTION
```

Acceptance:

- [ ] a clean Windows machine can install the produced signed installer — manual gate;
- [ ] upgrade preserves local state and the CNG App Manager identity — manual gate;
- [x] downgrade/rollback policy is documented and normal update policy blocks automatic downgrade;
- [x] unsigned test builds are clearly distinguishable from production;
- [x] production packaging refuses to proceed without signing credentials;
- [x] automated frontend/Rust/native packaging checks are green before merge.

---

## P10 — Software Updater

Sequencing exception recorded 2026-10-07: implementation may proceed while P9's clean-machine install and upgrade/state-preservation manual gates remain open. P9 must not be represented as fully accepted until those gates are proven.

Goals:

- [~] detect supported application updates;
- [~] verify publisher/source — official WinGet source trust is verified in V1; publisher evidence must remain explicit and may be unavailable until structured verification exists;
- [x] prefer trusted vendor/package-manager mechanisms — WinGet is the V1 provider;
- [x] never download executables from unverified URLs;
- [ ] real Windows representative update launch is manually verified.

V1 safety boundary:

- no arbitrary URL download;
- no Update All;
- no frontend-supplied command line or WinGet flags;
- exact package ID from a native opaque scan plan only;
- revalidate the candidate immediately before launch;
- official trusted `winget` source only;
- request interactive installer mode and never bypass installer hash checks.

---

## P11 — Driver Center

Only after P0–P10.

Principles:

- old does not automatically mean bad;
- prefer Windows Update/vendor sources;
- explain why an update is recommended;
- do not mass-update drivers blindly;
- create restore protection when feasible.

---

## P12 — Advanced Performance Optimizer

Only after the lower-risk foundation is mature.

Focus on evidence:

- boot-impact analysis;
- persistent resource consumers;
- optional application sleep policies;
- explainable service/task recommendations.

Avoid "disable everything" optimization.

---

## Working cadence

For each phase:

1. plan;
2. implement;
3. test;
4. inspect failures;
5. fix;
6. update docs;
7. merge;
8. create preview artifact where relevant;
9. proceed only after the acceptance gate passes.

The next implementation task after these planning documents are merged is **P0 — Repository foundation**.
