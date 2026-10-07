# AGENTS.md — PC Manager Desktop

This file is the persistent implementation contract for any AI coding agent working in this repository.

## 1. Mission

Build a production-grade native Windows system-management application that is safer, clearer, lighter, and more transparent than typical "PC cleaner" products.

The product must prioritize:

- safety before aggressive cleanup;
- measurable impact instead of fear-based warnings;
- reversibility and auditability;
- minimal privileges;
- native Windows behavior;
- predictable releases;
- clean integration with the existing App Manager platform.

## 2. Required stack

Unless a later architecture decision record explicitly changes it:

- UI: React + TypeScript
- Build UI: Vite
- Desktop shell: Tauri
- System engine: Rust
- Privileged component: separate Rust Windows Service
- Local database: SQLite
- Windows integration: Windows APIs first; PowerShell/WMI only where justified
- Tests: Rust unit/integration tests + TypeScript tests
- CI/CD: GitHub Actions
- Target: Windows 10/11 x64 first

Do not replace Tauri/Rust with Electron or a web-only implementation.

## 3. Repository shape

Target structure:

```text
pc-manager-desktop/
├─ apps/
│  └─ desktop/
│     ├─ src/
│     └─ src-tauri/
├─ crates/
│  ├─ pc-core/
│  ├─ pc-windows/
│  ├─ pc-monitor/
│  ├─ pc-updater/
│  └─ app-manager-client/
├─ services/
│  └─ windows-service/
├─ contracts/
├─ installer/
├─ scripts/
├─ tests/
├─ docs/
├─ .github/workflows/
├─ Cargo.toml
├─ package.json
└─ README.md
```

Keep UI, core logic, Windows-specific code, privileged operations, and App Manager networking separated.

## 4. Security rules — non-negotiable

1. The main UI process must not require permanent Administrator rights.
2. Privileged operations go through the Windows Service.
3. The privileged IPC surface must use an explicit allow-list of typed commands.
4. Never implement arbitrary remote shell, arbitrary PowerShell, arbitrary process execution, or arbitrary registry-write commands from App Manager.
5. Validate every privileged command on the service side, not only in the UI.
6. Default to deny when a request is malformed or unsupported.
7. Never silently delete user files.
8. Before destructive cleanup, calculate what will be removed and show it.
9. Risky optimizations must create rollback metadata when technically possible.
10. Secrets, API keys, signing keys, and private tokens must never be committed.
11. Do not upload personal filenames, browsing history, file contents, or document contents to App Manager.
12. Telemetry must be minimal, explicit, and privacy-preserving.

## 5. Product behavior rules

Do not use scareware language such as "CRITICAL" solely because software or drivers are old.

Every recommendation should expose, where possible:

- finding;
- evidence;
- expected benefit;
- risk level;
- what will change;
- whether it can be undone.

Examples:

- "6 startup apps can be disabled; estimated boot impact: medium."
- "8.4 GB can be reclaimed, including 5.7 GB temporary cache."
- "3 drivers have newer vendor releases; 1 is recommended due to a known stability fix."

Do not build a generic registry cleaner as a core feature. Registry changes are allowed only for specific, documented maintenance operations with a clear reason and rollback path.

## 6. App Manager integration

The desktop client uses a stable identity:

```json
{
  "appId": "pc-manager",
  "platform": "windows",
  "deviceType": "desktop-native"
}
```

App Manager may coordinate:

- device registration;
- device approval/blocking;
- heartbeat / online state;
- installed app version;
- release channel: dev / beta / stable;
- license/entitlement state;
- update availability;
- high-level non-destructive commands explicitly supported by the desktop app.

Allowed remote command examples:

- CHECK_UPDATE
- RUN_HEALTH_SCAN
- REFRESH_DEVICE_STATUS
- DISABLE_LICENSE

Remote commands must be versioned and strongly typed.

Never add EXECUTE_SHELL, RUN_POWERSHELL, RUN_COMMAND, or equivalents.

## 7. Core feature order

Implement in this order unless the plan is explicitly updated:

P0. Repository foundation and CI
P1. Native shell + design system + navigation
P2. System inventory + health data foundation
P3. Health Check
P4. Smart Cleaner + Restore Center
P5. Startup Manager
P6. Uninstaller + Duplicate Finder + Storage Analyzer
P7. System Monitor
P8. App Manager integration
P9. Signed packaging + updater + production hardening

Only after P0–P9 are stable:

P10. Software Updater
P11. Driver Center
P12. Advanced Performance Optimizer

## 8. Definition of done for every feature

A feature is not complete until:

- implementation exists;
- error states exist;
- loading/empty states exist;
- permission failures are handled;
- tests cover critical logic;
- destructive actions have confirmation;
- logs contain enough information to diagnose failures without leaking sensitive data;
- UI text explains impact accurately;
- documentation is updated;
- CI is green.

## 9. Work protocol for AI agents

Before coding:

1. Read this file.
2. Read `docs/MASTER_BUILD_PROMPT.md`.
3. Read `docs/IMPLEMENTATION_PLAN.md`.
4. Read `docs/ARCHITECTURE.md`.
5. Inspect existing code before creating duplicates.

For each task:

1. State the phase and acceptance criteria being addressed.
2. Make the smallest coherent implementation that advances that phase.
3. Do not mix unrelated refactors into the same change.
4. Run formatting, lint, type-check, tests, and build checks relevant to the change.
5. Fix failures before proceeding.
6. Update plan checkboxes only after evidence exists.
7. Do not claim a feature works if it is mocked.

Mocks are permitted only when clearly marked and when the next phase explicitly replaces them.

## 10. Release rules

Production releases must be reproducible and gated by CI.

Expected artifacts eventually include:

- Windows installer
- application executable/package
- checksums
- signed update metadata
- release notes

Preview and production must remain separate.

A production workflow must require an explicit confirmation input such as:

```text
confirm=DEPLOY_PRODUCTION
```

Never auto-promote a failed or unverified preview build to production.

## 11. UX direction

The product can be inspired by the clarity of mature utility apps, but must not copy CCleaner branding, artwork, copyrighted illustrations, logos, or pixel-identical screens.

Preferred UX:

- dark and light themes;
- compact sidebar;
- clear dashboard cards;
- restrained status colors;
- no fake urgency;
- native-feeling interactions;
- accessible keyboard navigation;
- scalable layout for 1366x768 through high-DPI displays.

## 12. Stop conditions

Stop implementation and report clearly when:

- a required Windows API behavior is uncertain and could cause data loss;
- a destructive operation cannot be made safe;
- credentials/signing material are required;
- App Manager API contracts are unavailable or incompatible;
- a requested feature would require arbitrary remote command execution;
- CI or build failures cannot be explained from available logs.

Do not bypass these constraints merely to make a demo appear successful.


## 13. Constitution 1.2 dependency sovereignty

This repository adopts `blueprint-os:universal-century-grade@1.2.0` at B4.

The mandatory operational-sovereignty rule is:

`LOCAL PRIVILEGED AUTHORITY → OPTIONAL REMOTE COORDINATION`

Non-negotiable consequences:

1. Cleanup, startup, uninstall, restore, monitor and other privileged machine actions remain locally authorized.
2. App Manager may coordinate registration, heartbeat, release/update metadata, entitlement and explicitly supported typed requests; it never becomes generic system authority.
3. Google Drive/Sheets/Apps Script or another cloud service may only be optional backup/sync/report adapters.
4. Core maintenance must remain usable during App Manager, Internet or cloud outages.
5. Remote generic shell, PowerShell, arbitrary process execution and arbitrary registry mutation remain forbidden.
6. Secrets, raw sensitive scan data and privileged state must not be stored in Drive/Sheets plaintext.
7. Any new external dependency must update `.blueprint/dependency-budget.json` with purpose, data boundary, degraded behavior and exit path.

Every AI coding task must preserve `.blueprint/constitution-adoption.json` and the dependency budget. Do not weaken the validator merely to make CI green.
