# P13 — RELEASE ACCEPTANCE PREFLIGHT & PRODUCT GAP CLOSURE

## Purpose

Close release-readiness gaps after the P0–P12 implementation foundations.
A green preview CI run is NOT evidence of a production-ready Windows application.

User direction 2026-10-08: continue non-destructive implementation and automated verification while previously deferred real-Windows manual gates remain recorded as pending. Production must stay blocked until those gates are genuinely verified.

## Mandatory P13 V1

1. Keep a machine-readable release acceptance register in `docs/release-acceptance.json`.
2. Fail closed on missing, duplicated, malformed or unknown gate IDs.
3. A gate can be accepted only with timestamped HTTPS evidence linking to its real-Windows verification record.
4. Validate schema and negative-path tests in regular CI, WITHOUT failing CI simply because manual gates remain pending.
5. Require every gate accepted as a separate production workflow step BEFORE signed packaging.
6. Preserve P9 exact production confirmation, protected environment, Authenticode signing, checksums and release-channel integrity.
7. An unsigned preview is not a public release. Do not auto-publish.
8. Do not create or falsify acceptance evidence.
9. A passing unit test that simulates an accepted manifest does NOT accept real gates.

## Broader remaining work, to be separately planned

- The Windows Service under `services/windows-service` remains a non-privileged skeleton. A privileged service is not to be activated until versioned typed IPC, caller authorization, timeout/size limits, service-side validation, audit, and rollback are designed and tested.
- SQLite is still a planned durable local persistence layer. Migrating existing app JSON/JSONL data requires transactional schema versioning, safe migration/backup, preservation of local history, and rollback tests.
- Investigate npm advisory output, distinguish shipped/runtime vs dev-only dependencies, apply narrowly targeted updates and retest before release.
- Complete all real-Windows acceptance checks from P5 through P12 (including P8 gateway and P9 signed install/upgrade) with evidence.
- Performance/CPU/RAM resource claims require real Windows idle and loaded machine validation before publishing efficacy claims.

## Acceptance

- `node scripts/check-release-acceptance.mjs --validate` passes with pending real-world gates;
- `node --test scripts/check-release-acceptance.test.mjs` passes;
- `node scripts/check-release-acceptance.mjs --require-ready` fails when any mandatory gate is pending;
- CI, Constitution, Rust, Windows build and preview remain green;
- Production workflow retains human signing/approval and is blocked by pending evidence;
- no Windows Service privileges, silent update, or remotely authorized machine mutation are introduced.
