# P5 EXECUTION PROMPT — Startup Manager

Read `/AGENTS.md`, `/docs/MASTER_BUILD_PROMPT.md`, `/docs/IMPLEMENTATION_PLAN.md`, and `/docs/ARCHITECTURE.md` before changes.

## Goal

Build a safe Windows Startup Manager that can inventory supported startup sources and enable/disable only entries that can be changed without exposing arbitrary registry, file, process, or PowerShell primitives.

P5 must reuse the same safety philosophy established in P4:

- typed operations;
- opaque/native entry IDs;
- revalidation before changes;
- operation audit;
- rollback-aware behavior;
- permission failures surfaced clearly;
- no permanent Administrator requirement.

## Sources

Inventory at least:

1. HKCU Run
2. HKCU RunOnce
3. HKLM Run
4. HKLM RunOnce
5. Current-user Startup folder
6. All-users Startup folder
7. selected Scheduled Tasks whose triggers are clearly AtLogOn or AtStartup

Entries that are visible but unsafe to change in the current process may be marked read-only.

## Domain model

Add startup-specific models in `pc-core`.

Suggested fields:

```text
StartupEntry
  id
  name
  source_type
  source_label
  command
  publisher?
  enabled
  can_change
  requires_elevation
  impact
  impact_evidence?
  detail

StartupChangeRequest
  entry_id
  enabled

StartupOperationRecord
  operation_id
  entry_id
  display_name
  source_type
  previous_enabled
  new_enabled
  success
  rollback_available
  completed_at_epoch_ms
  message
```

Do not expose raw registry mutation primitives.

## Stable native IDs

The frontend must not submit arbitrary registry paths, file paths, task names, or commands.

Generate opaque entry IDs from native-discovered identity, for example a hash of:

- source type
- source location
- value/file/task identity

When a toggle request arrives:

1. rediscover startup entries;
2. locate the opaque ID;
3. verify it still refers to the same supported source;
4. execute only the typed transition allowed for that source.

## Enable / disable behavior

### HKCU Run / RunOnce

Support safe enable/disable.

Disabling must preserve the original value in a PC Manager-owned backup location so it can be restored.

Recommended backup namespace:

```text
HKCU\Software\PCManager\StartupBackup\...
```

Re-enabling restores the backed-up value to its exact original supported Run/RunOnce key.

Do not allow arbitrary registry paths.

### HKLM Run / RunOnce

Inventory is required.

Changing HKLM entries may require elevation. Until the privileged Windows Service is ready, mark them read-only when modification would require elevation.

Do not auto-elevate the entire desktop application.

### Startup folders

Inventory current-user and all-users Startup folders.

For current-user items, disable by moving only a native-discovered startup file into a PC Manager-owned disabled-startup directory under LOCALAPPDATA.

Re-enable by moving it back only to its recorded original Startup folder.

Never accept arbitrary file paths from the frontend.

All-users Startup entries may be read-only when permission is insufficient.

### Scheduled Tasks

Inventory only tasks with clear startup/logon triggers.

Use a fixed native/provider implementation. If PowerShell is used, the command/script is authored entirely by PC Manager and arguments are derived from a revalidated native entry, never arbitrary script supplied by the UI.

Enable/disable only when the operation succeeds without requiring unsupported elevation. Permission failures must be reported, not bypassed.

## Impact

Do not fabricate startup impact.

P5 may use:

- `unknown` when no evidence exists;
- `low/medium/high` only if based on measurable evidence introduced by a supported method.

Do not infer impact from application name, publisher, or command length.

## Audit and rollback

Every successful change creates a local startup operation record.

The operation must include:

- what entry changed;
- previous state;
- new state;
- whether rollback is available;
- completion time;
- outcome.

Do not store secrets.

Registry/folder toggles should be reversible.

## UI

Replace the Startup placeholder with a functional page.

Required:

- refresh inventory;
- search/filter;
- source badge;
- publisher if known;
- command/details;
- enabled/disabled state;
- read-only state when unsupported;
- explicit confirmation before changing an item;
- exact result/error feedback;
- no bulk-disable button in P5 V1.

Make it clear that disabling startup does not uninstall the application.

## Tests

Add useful tests for:

- opaque ID stability;
- supported source mapping;
- Run/RunOnce backup/restore logic using isolated/disposable test data;
- startup folder move/restore using disposable files;
- unsupported/read-only source behavior;
- permission failure handling where practical;
- scheduled-task trigger filtering;
- frontend search/filter/state helpers.

Windows CI should include disposable smoke tests that do not modify persistent user startup state.

## Acceptance gate

P5 is complete only when:

- Run/RunOnce inventory works;
- Startup folder inventory works;
- selected AtLogOn/AtStartup Scheduled Tasks are represented;
- safe enable/disable works for supported entries;
- unsupported/elevated entries remain read-only;
- disabling never uninstalls the app;
- every successful change gets an operation record;
- reversible changes preserve enough state for restoration;
- frontend, Rust, Clippy, Windows tests, and native Windows build are green;
- a real Windows machine verifies inventory and a disposable enable/disable round-trip before proceeding to P6.

Use branch `feat/p5-startup-manager`.

Do not proceed to P6 until P5 is merged and verified on a real Windows machine.
