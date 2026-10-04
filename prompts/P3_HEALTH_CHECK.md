# P3 EXECUTION PROMPT — Health Check V1

Read `/AGENTS.md`, `/docs/MASTER_BUILD_PROMPT.md`, `/docs/IMPLEMENTATION_PLAN.md`, and `/docs/ARCHITECTURE.md` before changes.

## Goal

Implement an explainable, read-only Health Check V1 using the real P2 system inventory.

P3 must not modify Windows, delete files, disable startup items, install updates, or request Administrator privileges.

## Supported categories in P3

Health Check has five categories:

- Storage
- Performance
- Security
- Updates
- Privacy

P3 may only score categories backed by real evidence.

For categories not yet supported, return an explicit `unavailable` state with an explanation. Do not guess or fabricate a score.

## Domain model

Create platform-neutral health models in `pc-core`.

A report should include, at minimum:

```text
scan_id
collected_at_epoch_ms
score
coverage
category_results
findings
inventory_warnings
```

Each finding must include:

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

Recommended enums:

- category: storage / performance / security / updates / privacy
- status: good / attention / critical / unavailable
- severity: info / warning / critical
- risk: none / low / medium / high

## Scoring rules

Scoring must be deterministic, testable, and explainable.

Requirements:

- Unsupported categories do not reduce the score.
- Coverage must reveal how much of the five-category model is actually implemented.
- The UI must label the score as partial whenever coverage is below 100%.
- A category score must be derived only from findings in that category.
- Do not use arbitrary alarming language.

Initial supported checks may include:

### Storage

For each local fixed volume with a non-zero total size:

- free < 10% => critical finding
- free >= 10% and < 20% => warning finding
- otherwise no problem finding

Do not claim files can be safely deleted in P3.

### Performance

Use evidence already collected by P2 only.

Allowed examples:

- available RAM < 10% => critical memory-pressure finding
- available RAM >= 10% and < 20% => warning memory-pressure finding
- very large startup inventory may be reported as an informational/attention finding only if the threshold is documented and the wording is restrained

Do not infer CPU slowness from CPU model or process count.

### Security / Updates / Privacy

Return `unavailable` in P3 unless a concrete supported check is implemented with real evidence.

## Native command

Add a typed Tauri command:

```text
run_health_check()
```

The command should:

1. collect a fresh P2 system inventory;
2. evaluate it through `pc-core`;
3. return the health report or a structured error.

Do not duplicate Windows collection logic inside the UI.

## Cancellation

The P3 UI must allow a user to cancel an in-progress scan safely.

If the underlying Windows inventory call cannot be interrupted safely in this phase, cancellation may mean:

- mark the UI request as cancelled;
- ignore a late result;
- do not persist or render a cancelled result.

Make this behavior explicit in code/comments. Do not kill arbitrary processes.

## Persistence

Persist the latest completed Health Check locally.

For P3, a small local-only persistence layer is acceptable if it stores only the report and no sensitive file contents.

The UI must:

- restore the latest completed report on launch;
- never persist a cancelled or failed scan;
- offer a fresh rescan.

## UI

Replace the Health Check placeholder with a functional page.

Required states:

- idle / no scan yet
- scanning
- cancelled
- error with retry
- completed

Completed view must show:

- score
- coverage / partial-score label
- five category states
- findings
- evidence
- recommended action text
- unsupported categories as unavailable

No destructive buttons in P3.

The Overview page may show the latest Health Check summary, but it must never fabricate a score before a real scan exists.

## Tests

Add meaningful tests for:

- storage thresholds
- memory thresholds
- unsupported category behavior
- score / coverage calculation
- stable finding IDs
- frontend persistence helpers
- cancellation result-discard behavior where practical

All existing P0–P2 CI gates remain mandatory.

## Acceptance gate

P3 is complete only when:

- Scan Now performs real checks from a fresh P2 inventory;
- progress has a safe cancel path;
- completed results persist locally;
- a cancelled or failed scan is not persisted;
- score derives from concrete findings;
- unsupported categories are shown as unavailable;
- no destructive action occurs;
- no Administrator privilege is required;
- frontend checks pass;
- Rust checks pass;
- Windows native build passes;
- CI is green.

Use branch `feat/p3-health-check` and open a PR into `main`.

Do not proceed to P4 until P3 is merged and the Health Check behavior has been verified on a real Windows machine.
