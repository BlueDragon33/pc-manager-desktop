# P6 EXECUTION PROMPT — Apps, Duplicate Finder, Storage Analyzer

Read `/AGENTS.md`, `/docs/MASTER_BUILD_PROMPT.md`, `/docs/IMPLEMENTATION_PLAN.md`, and `/docs/ARCHITECTURE.md` before changes.

## Goal

Implement P6 as three separately verifiable slices:

1. P6A — Uninstaller
2. P6B — Duplicate Finder
3. P6C — Storage Analyzer

Do not mix destructive behavior across slices. Each slice must keep native paths/commands behind typed native operations and must fail closed when data changes between preview and execution.

P5 real-Windows inventory was observed, but its enable-disable-enable round-trip verification was explicitly deferred by the user. Do not falsely mark that P5 gate complete.

## P6A — Uninstaller

Enumerate installed programs from supported Windows uninstall registry locations (HKLM 64-bit, HKLM WOW6432Node, HKCU). Return a typed `InstalledProgram` with an opaque native ID, name, version, publisher, optional display-only install location/estimated size, `can_uninstall`, `requires_elevation`, and source label.

Do not expose registry paths or raw uninstall commands to the frontend. Generate the opaque ID from native-discovered hive + uninstall subkey and rediscover/revalidate by ID immediately before starting an uninstall.

V1 opens only the program's registered standard uninstall flow. No forced silent uninstall, no bulk uninstall, no frontend-supplied command/path, and do not prefer `QuietUninstallString`. Reject hidden/system components and entries without a supported uninstall registration. The vendor/Windows uninstaller controls the actual removal. Refresh inventory after the external flow.

UI requirements: loading/error/empty states, refresh, search/filter, count, source/elevation/read-only indicators, explicit confirmation, accurate result feedback.

Tests: opaque ID stability, deduplication/mapping, system/hidden filtering, search/filter helpers, unsupported/read-only behavior, frontend cannot supply an uninstall command, Windows inventory smoke test.

## P6B — Duplicate Finder

User chooses one or more roots through the desktop picker. Native code receives only picker-approved roots.

Scan stages: enumerate ordinary files → group by exact size → partial hash collision groups → full hash collision groups. Report duplicates only after full hash equality.

Skip symlinks and Windows reparse points. Inaccessible files become warnings. Support cancellation and explicit exclusions. Never follow junction/reparse loops. Revalidate metadata for files that may have changed.

Deletion is manual-selection only. Show exact selected files/bytes, require confirmation, revalidate root/path/file identity, never auto-select every copy, preserve at least one copy per group, and log the operation honestly.

## P6C — Storage Analyzer

Scan picker-approved roots and return folder tree aggregation, largest files, file-type/extension grouping, inaccessible-path warnings, explicit exclusions, progress and cancellation.

Storage Analyzer is read-only in P6C V1. It does not delete files. Skip symlinks/reparse points and tolerate inaccessible paths.

## Acceptance gate

P6A: real Windows inventory, search/filter, opaque/revalidated standard uninstall launch, no silent/bulk uninstall, refresh, all CI gates green.

P6B: full-hash proof before duplicate declaration, reparse/symlink loops skipped, inaccessible files tolerated, cancellation works, deletion is explicit and revalidated.

P6C: aggregation, largest-files, type grouping, exclusions, cancellation, read-only behavior, and real Windows scan on a disposable/user-selected folder.

Use branch `feat/p6-apps-duplicates-storage`.

Proceed continuously through P6A, P6B and P6C, but stop for real-machine verification only when native behavior cannot be proven safely in CI.
