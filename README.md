# PC Manager Desktop

Native Windows system-management application inspired by the useful parts of established PC utilities, but designed around safety, transparency, rollback, and centralized management.

## Product direction

PC Manager Desktop targets Windows 10/11 x64 and uses:

- React + TypeScript for the UI
- Tauri for the desktop shell
- Rust for system logic
- a separate Windows Service boundary for future elevated operations
- SQLite for local state, audit history, snapshots, and rollback metadata
- App Manager integration for device registration, heartbeat, release channels, licensing, and update coordination

## Governance baseline

- Universal Constitution: **blueprint-os:universal-century-grade@1.2.0**
- Blueprint level: **B4**
- Runtime posture: **LOCAL_CORE**
- Canonical dependency budget: `.blueprint/dependency-budget.json`
- Cloud/App Manager integrations are optional coordination layers, not privileged local authority.

## Core principles

1. Never exaggerate system problems to push upgrades.
2. Never delete or disable something without explaining the impact.
3. Every risky optimization must be reversible where technically possible.
4. The UI process should not run permanently as Administrator.
5. Privileged operations must be exposed through a narrow, allow-listed command surface.
6. Remote management must never allow arbitrary shell or PowerShell execution.
7. Every release must pass automated checks before production packaging.

## Planning source of truth

Read these before feature work:

- `AGENTS.md`
- `docs/MASTER_BUILD_PROMPT.md`
- `docs/IMPLEMENTATION_PLAN.md`
- `docs/ARCHITECTURE.md`
- the active phase prompt under `prompts/`

## Current phase

**P10 — Software Updater**

P0–P9 implementation foundations are present, while P9 still retains its documented manual clean-machine install and upgrade/state-preservation gates. P10 adds conservative software-update discovery and one-at-a-time launch through the verified official WinGet source; arbitrary URLs, custom command lines, silent mass updates, force flags, and hash bypasses remain prohibited.

## Prerequisites

For local development on Windows:

- Windows 10 or 11 x64
- Node.js 22+
- npm
- Rust stable toolchain with Cargo
- Microsoft C++ Build Tools required by Tauri/WebView2 development
- WebView2 runtime

Follow the official Tauri Windows prerequisites if the machine has not built Tauri applications before.

## Install

From the repository root:

```powershell
npm ci
cargo check --workspace --locked
```

The npm and Cargo lockfiles are committed so CI and developer builds resolve the same dependency graph.

## Run in development

```powershell
npm run dev
```

This launches the Vite frontend through the native Tauri shell.

## Build

Frontend only:

```powershell
npm run build:frontend
```

Native application without installer packaging:

```powershell
npm run build
```

## Checks

Frontend:

```powershell
npm run lint
npm run typecheck
npm run test
npm run format:check
npm run build:frontend
```

Rust:

```powershell
cargo fmt --check
cargo check --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

On Windows, the combined helper is:

```powershell
./scripts/check.ps1
```

## Initial release target

The first useful release will focus on:

- Health Check
- Smart Cleaner
- Startup Manager
- Uninstaller
- Duplicate Finder
- Storage Analyzer
- System Monitor
- Restore Center
- App Manager connectivity

Software Updater, Driver Center, and deeper Performance Optimizer features come only after the safety foundation is proven.

## Status

The repository is under active construction and is not production-ready.
