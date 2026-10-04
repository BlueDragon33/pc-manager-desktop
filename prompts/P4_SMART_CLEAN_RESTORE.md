# P4 EXECUTION PROMPT — Smart Cleaner + Restore Center Foundation

Read `/AGENTS.md`, `/docs/MASTER_BUILD_PROMPT.md`, `/docs/IMPLEMENTATION_PLAN.md`, and `/docs/ARCHITECTURE.md` before changes.

## Goal

Build the first real maintenance subsystem without jumping directly to deletion.

P4 is deliberately split into two gates:

- **P4A — scan + preview + cleanup plan foundation (read-only)**
- **P4B — execution + audit + rollback-capability records**

P4A must be completed, merged, and verified on a real Windows machine before P4B is allowed to delete or modify anything.

---

# P4A — Smart Cleaner scan foundation

## Required providers

Use explicit, documented roots only.

### Windows user temp

Scan only known user temp roots such as:

- `%TEMP%`
- `%LOCALAPPDATA%\Temp`

Deduplicate roots when they resolve to the same directory.

For the first implementation, only files old enough to be considered stale should be eligible. Use a conservative documented age threshold such as 24 hours.

### Browser cache

Support explicit cache roots for installed Chromium browsers where present:

- Google Chrome
- Microsoft Edge

Do not scan browser history, cookies, saved passwords, form data, bookmarks, sessions, or user profile contents outside the cache subdirectories.

### Application cache

Support a small allow-list of explicit application cache roots where present, for example:

- Visual Studio Code cache
- Discord cache
- Slack cache

Do not invent generic `AppData` wildcard rules.

### Recycle Bin

Represent Recycle Bin as a separate opt-in provider.

P4A may report it as unavailable if a reliable read-only size/count implementation is not yet present. Do not treat it as enabled by default.

## Filesystem safety

- Never recurse through symlinks, junctions, or Windows reparse points.
- Never leave a provider's declared root.
- Ignore inaccessible files and record provider warnings instead of aborting the whole scan.
- Do not follow arbitrary paths supplied by the UI.
- Do not scan document folders, Desktop, Downloads, Pictures, Videos, source repositories, or user-created folders in P4A.
- Do not delete anything in P4A.

## Domain model

Add platform-neutral models in `pc-core`.

Suggested structures:

```text
CleanupCategory
CleanupProviderSummary
CleanupScanSummary
CleanupPlan
CleanupPlanItem
CleanupWarning
```

The internal cleanup plan must contain enough information for P4B to revalidate every candidate before deletion.

The UI-facing scan summary should include:

```text
plan_id
collected_at_epoch_ms
total_bytes
total_files
providers[]
warnings[]
execution_available = false
```

Each provider summary should include:

```text
provider_id
display_name
category
enabled_by_default
available
file_count
bytes
reversible
description
warnings[]
```

Do not return thousands of raw paths to the UI.

## Plan storage

P4A should create a cleanup plan and retain it inside the native process for later execution.

The UI only receives the plan ID and aggregate summary.

A future execution command must use the plan ID, not arbitrary paths from the frontend.

## Native command

Add a typed command such as:

```text
scan_cleanup_candidates(options)
```

The command must:

1. resolve only built-in provider roots;
2. scan read-only;
3. build a cleanup plan;
4. store the plan natively;
5. return a summary.

No delete command is permitted in P4A.

## UI

Replace the Smart Clean placeholder with a real preview page.

Required states:

- no scan
- scanning
- cancelled
- error
- completed

Completed view must show:

- exact total bytes
- file count
- provider-by-provider totals
- provider warnings
- "Preview only" / "Cleaning disabled until safety verification" messaging
- no active Clean button

Allow the Recycle Bin opt-in toggle to exist, but if the provider is unavailable it must be clearly marked.

## Tests

Add tests for:

- root deduplication
- stale temp age threshold
- reparse/symlink skip behavior where testable
- exact byte aggregation
- inaccessible-file warning behavior where practical
- provider summary aggregation
- plan IDs and plan lookup
- frontend scan-state helpers

All P0–P3 CI gates remain mandatory.

## P4A acceptance

P4A is complete only when:

- real Windows directories are scanned;
- exact bytes/files are returned;
- no arbitrary root is accepted from the UI;
- no file is deleted;
- reparse points are skipped;
- provider failures do not abort unrelated providers;
- cleanup plan is stored natively and referenced by plan ID;
- Smart Clean UI is functional and clearly read-only;
- frontend/Rust/Windows CI is green;
- a real Windows machine verifies the preview totals look plausible.

Stop here for user verification.

---

# P4B — Cleanup execution + Restore Center foundation

P4B begins only after P4A is verified.

## Execution rules

Execution must accept only:

```text
execute_cleanup_plan(plan_id)
```

Never accept arbitrary paths or shell commands.

Before deleting each item:

1. re-resolve/canonicalize the path;
2. confirm it remains inside the original provider root;
3. confirm it is not a reparse point;
4. confirm the file still matches the planned candidate class;
5. attempt deletion;
6. record success/failure per item;
7. continue after unrelated failures.

Prefer safe deletion semantics and user-level operations.

Do not require permanent Administrator rights.

System locations requiring elevation must remain unsupported until the Windows Service IPC layer is ready.

## Operation log

Every execution creates an immutable local operation record containing:

- operation_id
- plan_id
- started/completed time
- requested bytes/files
- deleted bytes/files
- failed items count
- provider results
- rollback capability
- errors

Do not log sensitive file contents.

## Restore Center

Be explicit:

- ordinary cache deletion is usually **not reversible**;
- actions with no safe rollback must say "Not restorable";
- future reversible changes can provide rollback records.

Restore Center in P4B is therefore first an operation/audit center, not a fake undelete feature.

## P4B acceptance

- no arbitrary path deletion;
- every path revalidated;
- partial failures are isolated;
- operation audit is persisted locally;
- UI requires explicit confirmation;
- exact execution result is shown;
- non-restorable operations are labeled honestly;
- CI is green;
- real Windows test is performed with intentionally disposable temp/cache data.

Use separate implementation branches for P4A and P4B.
