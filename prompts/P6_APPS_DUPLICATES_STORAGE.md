# P6 EXECUTION PROMPT — Apps, Duplicate Finder, Storage Analyzer

Read `/AGENTS.md`, `/docs/MASTER_BUILD_PROMPT.md`, `/docs/IMPLEMENTATION_PLAN.md`, and `/docs/ARCHITECTURE.md` before changes.

## Sequencing note

P5 implementation and CI are complete. Real-Windows startup inventory has been observed, but the disposable enable-disable-enable round trip was explicitly deferred by the user on 2026-10-06. Do not mark P5 fully accepted. This prompt authorizes P6 implementation while that single P5 verification item remains open.

## Goal

Replace the Apps, Duplicates, and Storage placeholders with real Windows functionality while preserving the repository safety contract:

- inventory before mutation;
- no arbitrary command execution;
- no silent uninstall;
- no automatic duplicate deletion;
- no traversal through symlink/reparse-point boundaries;
- inaccessible paths become bounded warnings rather than crashes;
- user-controlled scans are cancellable in the UI;
- native operations accept typed IDs or validated roots, not arbitrary executable commands supplied by the frontend.

Implement P6 in three coherent slices and keep each slice independently testable.

## P6A — Apps / Uninstaller

### Inventory

Reuse or extend the trusted installed-application provider from P2.

Expose an application-specific model sufficient for the Apps page:

```text
InstalledAppEntry
  id
  display_name
  publisher?
  version?
  install_location?
  install_date?
  estimated_size_bytes?
  source
  can_uninstall
  requires_elevation
  uninstall_kind
  detail
```

Use a stable opaque ID derived from native-discovered identity. The frontend must never submit an uninstall command line.

### Standard uninstall flow

V1 must launch only a native-discovered uninstall action after rediscovering the selected opaque application ID.

Rules:

1. Rediscover installed applications.
2. Resolve the opaque ID.
3. Verify the uninstall metadata still belongs to that same application.
4. Launch the registered standard uninstall flow.
5. Never append frontend-supplied arguments.
6. Never force silent flags such as `/quiet`, `/S`, or MSI `/qn`.
7. Do not auto-elevate the entire desktop app.
8. If elevation is required and cannot be safely delegated yet, surface that state clearly.
9. Refresh inventory after the launched uninstaller exits where practical, otherwise provide an explicit Refresh action.

Do not expose a generic process-launch or shell command API.

### Apps UI

Required:

- real inventory;
- search/filter;
- publisher/version/source when known;
- uninstall availability/read-only state;
- explicit confirmation;
- clear message that PC Manager opens the application's standard uninstall flow;
- Refresh;
- loading/error/empty states.

## P6B — Duplicate Finder

### Scan contract

The user selects one or more allowed directory roots through the native folder-selection UI or another Tauri path-selection mechanism. Do not scan arbitrary roots received from remote/App Manager commands.

Support user exclusions and built-in safety exclusions.

Traversal rules:

- skip symlinks and Windows reparse points;
- do not cross outside selected canonical roots;
- isolate access-denied/inaccessible paths into warnings;
- bound concurrency and memory;
- ignore files smaller than a documented minimum size unless a later setting enables them;
- cancellation must prevent publishing stale results.

### Duplicate proof pipeline

A duplicate must be proven in stages:

1. group regular files by exact byte size;
2. discard singleton size groups;
3. compute a bounded partial hash for candidates;
4. discard singleton partial-hash groups;
5. compute a full cryptographic hash of every remaining candidate;
6. declare duplicates only when exact size and full hash match.

Never declare duplicates based on filename, extension, timestamps, or partial hash alone.

Suggested models:

```text
DuplicateScanOptions
DuplicateFile
DuplicateGroup
DuplicateScanSummary
DuplicateWarning
DuplicateDeleteRequest
DuplicateDeleteResult
```

### Deletion

Deletion is user-selected only.

Before deleting each file:

- resolve the native scan result/opaque ID;
- revalidate canonical path under the originally selected root;
- reject symlink/reparse points;
- verify current size and full hash still match the scan result;
- delete only that selected file;
- failure of one file must not abort unrelated selected files;
- record a local operation result without persisting unnecessary personal paths.

No "delete all duplicates" default. Preserve at least one copy per group in the UI by default and make the user choose deletions explicitly.

## P6C — Storage Analyzer

Provide real directory analysis for user-selected roots.

Required output:

- total scanned bytes/files;
- tree/folder aggregation;
- largest files;
- useful file-type/extension grouping;
- warnings for inaccessible paths;
- exclusions;
- cancellation.

Use the same safe traversal foundation as Duplicate Finder so reparse/symlink handling is not duplicated inconsistently.

The analyzer is read-only in P6.

Suggested models:

```text
StorageScanOptions
StorageFolderAggregate
StorageFileEntry
StorageTypeAggregate
StorageScanSummary
StorageWarning
```

Bound result lists: return top-N largest files/folders rather than millions of rows.

## Shared native scan engine

Prefer a reusable Windows filesystem walker for Duplicate Finder and Storage Analyzer with:

- canonical root validation;
- direct reparse/symlink rejection;
- bounded warning collection;
- cancellation token/generation where feasible;
- deterministic aggregation helpers;
- no panics on malformed or disappearing files.

Keep platform-neutral grouping/hash/domain logic in `pc-core` where practical and Windows filesystem acquisition in `pc-windows`.

## Tauri command surface

Commands must be typed and feature-specific. Good examples:

```text
list_installed_apps()
launch_uninstall(app_id)
scan_duplicates(options)
delete_duplicate_files(request)
scan_storage(options)
```

Do not add:

```text
run_command(...)
run_powershell(...)
delete_file(path)
scan_any_path_from_remote(...)
```

## Tests

Add meaningful tests for:

### Apps
- stable opaque application IDs;
- search/filter helpers;
- unsupported/read-only uninstall records;
- rediscovery/revalidation mapping;
- no silent-uninstall argument injection.

### Duplicates
- size grouping;
- partial hash grouping;
- full hash verification;
- same-name but different-content files are not duplicates;
- same partial hash but different full content is not accepted;
- changed-after-scan candidate is rejected before deletion;
- symlink/reparse handling where practical;
- inaccessible path warning isolation.

### Storage
- tree aggregation;
- type grouping;
- largest-file ordering;
- exclusions;
- cancellation/stale-result helpers;
- bounded top-N output.

Use disposable temporary directories in tests. Never delete persistent user data in CI.

## UI quality

All three pages must:

- work within the existing scrollable content shell;
- remain usable at 1366x768;
- have loading/error/empty states;
- avoid content overflow;
- expose honest unsupported/read-only states;
- avoid scareware language.

## Acceptance gate

P6 is implementation-complete only when:

- Apps enumerates real installed software and can open a revalidated standard uninstall flow without silent flags;
- Apps search/filter and refresh work;
- Duplicate Finder scans user-selected roots with exclusions;
- duplicate groups require full-hash equality;
- only explicitly selected duplicate files can be deleted;
- every deletion candidate is revalidated immediately before deletion;
- Storage Analyzer reports real totals, tree aggregation, largest files, and type grouping;
- shared traversal skips symlink/reparse boundaries;
- inaccessible paths become warnings, not crashes;
- frontend lint/typecheck/tests/build/format pass;
- Rust fmt/check/test/Clippy pass;
- Windows native build and disposable filesystem tests are green;
- documentation is updated with known limitations.

Do not claim real-user-data destructive verification unless it was actually performed.

Use branch `feat/p6-apps-duplicates-storage`.
