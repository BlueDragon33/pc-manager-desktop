# PC Manager Desktop

Native Windows system-management application inspired by the useful parts of tools such as CCleaner, but designed around safety, transparency, rollback, and centralized management.

## Product direction

PC Manager Desktop is a Windows 10/11 x64 desktop application built with:

- React + TypeScript for the UI
- Tauri for the desktop shell
- Rust for system logic
- A separate privileged Windows Service for operations that require elevation
- SQLite for local state, audit history, snapshots, and rollback metadata
- App Manager integration for device registration, heartbeat, release channels, licensing, and update coordination

## Core principles

1. Never exaggerate system problems to push upgrades.
2. Never delete or disable something without explaining the impact.
3. Every risky optimization must be reversible where technically possible.
4. The UI process should not run permanently as Administrator.
5. Privileged operations must be exposed through a narrow, allow-listed command surface.
6. Remote management must never allow arbitrary shell or PowerShell execution.
7. Every release must pass automated checks before production packaging.

## Planning source of truth

The repository will use:

- `AGENTS.md` — persistent implementation instructions for AI coding agents.
- `docs/MASTER_BUILD_PROMPT.md` — complete product/build prompt.
- `docs/IMPLEMENTATION_PLAN.md` — phased roadmap and acceptance gates.
- `docs/ARCHITECTURE.md` — target architecture and module boundaries.

Do not start large feature work before reading those files.

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

Software Updater, Driver Center, and deeper Performance Optimizer features come after the safety foundation is proven.

## Status

Repository initialized. Architecture and implementation plan are being established before product code is generated.
