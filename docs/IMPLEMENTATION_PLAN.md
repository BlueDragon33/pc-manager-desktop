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

- [ ] Rust workspace
- [ ] React + TypeScript + Vite desktop UI
- [ ] Tauri desktop shell
- [ ] workspace-level formatting/lint commands
- [ ] basic unit-test commands
- [ ] `.editorconfig`
- [ ] sensible `.gitignore`
- [ ] GitHub Actions CI for Rust + TypeScript
- [ ] architecture docs wired into README
- [ ] initial app metadata: `appId=pc-manager`

Acceptance:

- clean clone can install/build;
- UI opens as a native desktop app;
- CI passes on main;
- no feature logic yet beyond a shell.

---

## P1 — Application shell and design system

Goal: create the durable UI frame.

Tasks:

- [ ] title bar/window strategy
- [ ] sidebar navigation
- [ ] Overview
- [ ] Health Check
- [ ] Smart Clean
- [ ] Startup
- [ ] Apps
- [ ] Storage
- [ ] Duplicates
- [ ] Monitor
- [ ] Restore
- [ ] Settings
- [ ] dark theme
- [ ] light theme
- [ ] keyboard/focus behavior
- [ ] responsive support down to 1366×768
- [ ] shared cards, badges, dialogs, progress states

Acceptance:

- navigation is functional;
- no copyrighted CCleaner assets are copied;
- no fake system numbers are presented as real data.

---

## P2 — System inventory foundation

Goal: establish trustworthy read-only Windows data.

Implement typed models for:

- [ ] OS/version
- [ ] hostname/device ID abstraction
- [ ] CPU summary
- [ ] RAM summary
- [ ] disk volumes
- [ ] free/used storage
- [ ] processes summary
- [ ] installed applications
- [ ] startup sources
- [ ] basic network adapters

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
- no Administrator requirement for normal inventory.

---

## P3 — Health Check V1

Goal: produce an explainable health report.

Categories:

- [ ] Storage
- [ ] Performance
- [ ] Security
- [ ] Updates
- [ ] Privacy

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

- Scan Now performs real checks;
- progress can be cancelled safely;
- results are persisted locally;
- score derives from findings;
- no destructive action occurs during scan.

---

## P4 — Smart Cleaner + Restore Center foundation

Goal: first write/destructive subsystem, built safely.

Smart Cleaner V1:

- [ ] Windows temp candidates
- [ ] application temp providers with explicit rules
- [ ] browser cache providers with explicit rules
- [ ] recycle bin as a separate opt-in item
- [ ] scan-only preview
- [ ] cleanup plan
- [ ] execution result

Restore foundation:

- [ ] operation log
- [ ] before-state metadata
- [ ] rollback capability flags
- [ ] rollback UI

Important:

Not every deleted cache file is restorable. The app must be honest about that. "Restore Center" records operations and restores only operations for which a safe rollback exists.

Acceptance:

- exact byte estimate before cleanup;
- user can inspect categories;
- no arbitrary filesystem wildcard deletion;
- errors do not abort unrelated cleanup items;
- audit record is stored.

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
