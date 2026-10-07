# P10 — SOFTWARE UPDATER

## Phase intent

Implement a conservative Windows software updater after the P9 packaging foundation.

P9 manual install/upgrade gates remain open. This sequencing exception allows P10 implementation to proceed without representing P9 as fully accepted.

## Safety contract

1. Prefer Windows Package Manager (WinGet) as the initial trusted provider.
2. Never accept an arbitrary download URL from the frontend or App Manager.
3. Never add a generic download-and-run primitive.
4. Never use `--ignore-security-hash`, `--force`, arbitrary `--custom`, or arbitrary `--override`.
5. The UI may select only an opaque candidate ID returned by the native scan.
6. Before launch, rediscover the candidate and verify the same package ID/version transition is still available.
7. Restrict V1 to the official `winget` source after validating its exported identity/trust metadata.
8. Launch only an exact package ID with a fixed, application-authored argument list.
9. Do not force silent installers. Request interactive installer mode and fail closed when package/source agreements cannot be handled non-interactively.
10. Do not auto-update all applications.

## P10 V1 scope

### Detection
- Detect whether WinGet is available.
- Verify the configured `winget` source is the official Microsoft source and reports a trusted source level.
- Enumerate upgrade candidates from `winget upgrade --source winget`.
- Parse the table by separator-column positions, not localized header names.
- Return name, exact package ID, installed version, available version, source and evidence.
- If publisher cannot be independently verified from structured provider data, say so explicitly rather than guessing.

### Update plan
- Create an in-memory native scan/plan with an opaque scan ID and candidate IDs.
- The frontend never supplies command lines, URLs or package-manager flags.
- Stale/missing plans fail closed.

### Launch
- Re-scan immediately before launch.
- Require the same exact package ID and available version to still be offered by the trusted source.
- Launch:
  `winget upgrade --id <fixed-id> --exact --source winget --interactive --accept-source-agreements --disable-interactivity`
- Do not pass `--accept-package-agreements`; packages requiring additional agreement should fail safely instead of being silently accepted.
- Return a typed launch result.

### UI
- Add an Updates navigation page.
- Show trusted-source state and provider availability.
- Show exact current -> available versions.
- Explain publisher verification honestly.
- Require explicit confirmation for one selected app.
- Refresh after launch.
- No "Update all" button in V1.

### App Manager
- Upgrade CHECK_UPDATE from the P8 placeholder to a real privacy-bounded aggregate result.
- It may trigger a local read-only scan.
- Return counts/provider/trust state only; do not return package IDs, app names, local paths or command lines.

## Acceptance criteria

- Rust unit tests cover:
  - trusted source validation;
  - table parsing with spaces in app names;
  - malformed/no-update output;
  - opaque candidate IDs;
  - revalidation mismatch rejection.
- Windows CI runs a non-destructive provider smoke test when WinGet is available; lack of WinGet is reported as unavailable, not fabricated.
- Frontend lint/typecheck/tests/build and Rust fmt/check/tests/Clippy are green.
- No arbitrary remote command, URL download, shell string, or frontend-provided package-manager flag exists.
- Manual real-Windows gate remains open for launching one representative update and confirming the expected vendor installer/UAC behavior.
