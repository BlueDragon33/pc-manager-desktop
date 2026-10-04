# P2 EXECUTION PROMPT — System Inventory Foundation

Read `/AGENTS.md`, `/docs/MASTER_BUILD_PROMPT.md`, `/docs/IMPLEMENTATION_PLAN.md`, and `/docs/ARCHITECTURE.md` before changes.

## Goal

Implement trustworthy, **read-only** Windows system inventory and expose it through the existing Tauri bridge.

P2 must not clean, delete, disable, uninstall, update, optimize, or elevate anything.

## Required inventory models

Define platform-neutral serialized models in `pc-core` for:

- OS/version
- hostname and local device identity abstraction
- CPU summary
- RAM summary
- disk volumes and free/used storage
- process summary
- installed applications
- startup sources
- basic network adapters
- structured inventory errors / unavailable fields

Keep raw Windows types out of `pc-core`.

## Windows implementation

Implement the concrete collectors in `pc-windows`.

Preferred approach:

- documented Rust/system APIs for general machine inventory;
- Windows Registry reads for installed applications, MachineGuid-derived local identity, and startup registry entries;
- standard Startup folders for startup-file discovery;
- no PowerShell unless a specific capability cannot be implemented safely otherwise.

Rules:

- Registry access is read-only.
- Do not expose raw MachineGuid. Derive a one-way local device identifier.
- Do not require Administrator.
- Inaccessible sources return partial inventory or structured errors rather than crashing.
- Deduplicate installed-application records conservatively.
- Do not execute uninstall strings or startup commands.
- Do not enumerate personal file contents.

## Native bridge

Add a typed Tauri command such as:

```text
get_system_inventory()
```

It returns a serialized inventory snapshot.

The existing `get_app_info()` command must remain supported.

## UI

Use real P2 inventory in the existing P1 shell.

At minimum, Overview should display a compact device snapshot when the native command succeeds:

- OS
- CPU
- memory
- primary/available volumes summary
- process count
- installed application count
- startup item count
- network adapter count

Loading, partial/unavailable, and error states must be explicit.

Do not invent values in browser-only development mode. If the native bridge is unavailable, say so.

Feature pages may remain non-mutating. It is acceptable to show read-only summaries on Apps/Startup/Storage if the implementation remains small and coherent.

## Tests

Add unit tests for:

- model serialization or constructors;
- byte/size calculation helpers;
- stable device-ID derivation helper using test input;
- registry/parser/dedup helpers where they can be tested without accessing the host registry.

Existing frontend and Rust checks must remain green.

## Acceptance gate

P2 is complete only when:

- OS/version is real on Windows;
- hostname/device identity abstraction is implemented;
- CPU and RAM summaries are real;
- disk volume/free-space inventory is real;
- process summary is real;
- installed applications are read from Windows sources;
- startup sources are read without modification;
- network adapters are enumerated;
- permission/source failures are handled;
- no Administrator rights are required for ordinary inventory;
- no mutation code is added;
- Overview renders real inventory or an honest unavailable state;
- frontend lint/typecheck/tests/build pass;
- Rust fmt/check/tests/Clippy pass;
- Windows native build passes;
- CI is green.

Use branch `feat/p2-system-inventory` and open a PR into `main`.

Do not proceed to P3 until P2 is merged with green CI.
