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

- [x] operation log
- [x] before-state summary metadata via immutable plan ID, requested file count, and requested bytes
- [x] rollback capability flags
- [x] rollback UI / operation-history UI with explicit "Not restorable" state

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
- [x] user verified plausible real-machine preview totals and full-page scrolling before P4B.

P4B execution gate:
- [x] every planned path is revalidated immediately before deletion;
- [x] errors do not abort unrelated cleanup items;
- [x] audit record is stored locally after completed execution;
- [x] execution result is shown accurately;
- [x] restore capability is labeled honestly;
- [x] Windows CI executes cleanup against intentionally disposable test data only;
- [x] frontend, Rust, Clippy, disposable Windows execution test, and native Windows build are green.

---

## P5 — Startup Manager

Goal: safe startup control.

Sources:

- [ ] supported Run/RunOnce registry locations
- [ ] startup folders
- [ ] selected scheduled tasks when they clearly represent startup behavior

Capabilities:

- [ ] inventory
- [ ] enable/disable where safe
- [ ] publisher/source display
- [ ] command display
- [ ] rollback record
- [ ] search/filter

Acceptance:

- changes survive reboot semantics correctly;
- disabling does not delete the underlying application;
- every change has an operation record.

---

## P6 — Apps, Duplicates, Storage

### Uninstaller

- [ ] enumerate installed programs
- [ ] search/filter
- [ ] open standard uninstall flow
- [ ] refresh after uninstall
- [ ] no forced silent uninstall in V1

### Duplicate Finder

- [ ] folder selection
- [ ] exclusions
- [ ] size grouping
- [ ] partial hashing
- [ ] full hashing
- [ ] preview
- [ ] user-selected deletion only

### Storage Analyzer

- [ ] tree aggregation
- [ ] largest files
- [ ] file-type grouping
- [ ] exclusions
- [ ] cancellation

Acceptance:

- symlink/reparse-point loops are handled;
- inaccessible paths do not crash scans;
- duplicates are only declared after full hash verification.

---

## P7 — System Monitor

Goal: low-overhead useful telemetry.

- [ ] CPU
- [ ] RAM
- [ ] disk activity where reliable
- [ ] network
- [ ] process top consumers
- [ ] GPU where reliable
- [ ] sensor capability detection

Acceptance:

- unsupported sensors show "Unavailable";
- sampling interval configurable;
- monitor can be paused;
- idle overhead measured and documented.

---

## P8 — App Manager integration

Goal: connect PC Manager to the existing administration platform.

Client capabilities:

- [ ] device identity
- [ ] registration
- [ ] approval state
- [ ] heartbeat
- [ ] online/offline semantics
- [ ] app version
- [ ] release channel
- [ ] entitlement/license state
- [ ] update policy
- [ ] typed remote command envelope

Remote command allow-list initially:

- [ ] CHECK_UPDATE
- [ ] RUN_HEALTH_SCAN
- [ ] REFRESH_DEVICE_STATUS
- [ ] DISABLE_LICENSE

Explicitly prohibited:

- arbitrary shell;
- arbitrary PowerShell;
- arbitrary process execution;
- arbitrary registry mutation;
- arbitrary file download-and-run.

Acceptance:

- offline mode works;
- retries use backoff;
- authentication is not stored as plaintext when avoidable;
- server cannot turn a typed command into arbitrary OS execution.

---

## P9 — Production packaging and update system

Goal: reliable release pipeline.

- [ ] preview build workflow
- [ ] production workflow
- [ ] production confirmation input
- [ ] Windows installer
- [ ] checksums
- [ ] signing integration hooks
- [ ] release notes
- [ ] updater metadata
- [ ] stable/beta/dev policy
- [ ] rollback/failed-update strategy

Desired production trigger pattern:

```powershell
gh workflow run production-release.yml --ref main -f confirm=DEPLOY_PRODUCTION
```

Acceptance:

- a clean Windows machine can install;
- upgrade preserves local state;
- downgrade/rollback policy documented;
- unsigned test builds are clearly distinguishable from production.

---

## P10 — Software Updater

Only after P0–P9.

Goals:

- detect supported application updates;
- verify publisher/source;
- prefer trusted vendor/package-manager mechanisms;
- never download executables from unverified URLs.

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
