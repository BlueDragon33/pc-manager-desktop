# IMPLEMENTATION PLAN

This is the execution roadmap for PC Manager Desktop.

Legend:

- [ ] not started
- [~] in progress
- [x] complete only after acceptance criteria are proven

---

## P0 — Repository foundation

Goal: create a reproducible monorepo before feature development.

Tasks:

- [x] Rust workspace
- [x] React + TypeScript + Vite desktop UI
- [x] Tauri desktop shell
- [x] workspace-level formatting/lint commands
- [x] basic unit-test commands
- [x] `.editorconfig`
- [x] sensible `.gitignore`
- [x] GitHub Actions CI for Rust + TypeScript
- [x] architecture docs wired into README
- [x] initial app metadata: `appId=pc-manager`

Acceptance:

- clean clone can install/build;
- UI opens as a native desktop app;
- CI passes on main;
- no feature logic yet beyond a shell.

---

## P1 — Application shell and design system

Goal: create the durable UI frame.

Tasks:

- [x] title bar/window strategy — retain native Tauri/Windows chrome for the initial release
- [x] sidebar navigation
- [x] Overview
- [x] Health Check
- [x] Smart Clean
- [x] Startup
- [x] Apps
- [x] Storage
- [x] Duplicates
- [x] Monitor
- [x] Restore
- [x] Settings
- [x] dark theme
- [x] light theme
- [x] keyboard/focus behavior
- [x] responsive support down to 1366×768
- [x] shared card/badge states; destructive dialogs and progress flows are completed in the first phases that need them

Acceptance:

- navigation is functional;
- no copyrighted CCleaner assets are copied;
- no fake system numbers are presented as real data;
- Windows native build and CI are green;
- user approved the P1 visual direction before P2 began.

---

## P2 — System inventory foundation

Goal: establish trustworthy read-only Windows data.

Implement typed models for:

- [x] OS/version
- [x] hostname/device ID abstraction
- [x] CPU summary
- [x] RAM summary
- [x] disk volumes
- [x] free/used storage
- [x] processes summary
- [x] installed applications
- [x] startup sources
- [x] basic network adapters

Architecture:

```text
UI
 ↓
Tauri commands
 ↓
pc-core interfaces
 ↓
pc-windows implementations
```

Acceptance:

- read-only;
- handles permission/access failures;
- unit tests for parsing/mapping logic;
- no Administrator requirement for normal inventory;
- Windows CI executes the real inventory provider and verifies hostname, OS, memory, CPU, and local-volume data;
- frontend, Rust, and Windows native build gates are green.

---

## P3 — Health Check V1

Goal: produce an explainable health report.

Categories:

- [x] Storage
- [x] Performance
- [x] Security — explicit unavailable state in V1 until an evidence-backed check is implemented
- [x] Updates — explicit unavailable state in V1 until trusted update checks are implemented
- [x] Privacy — explicit unavailable state in V1 until an evidence-backed check is implemented

V1 may have limited Security/Updates/Privacy coverage, but unsupported checks must be shown as unavailable rather than guessed.

Each finding:

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

Acceptance:

- Scan Now performs real checks from a fresh P2 Windows inventory;
- progress can be cancelled safely by invalidating the UI request and discarding any late native result;
- the latest completed result is persisted locally; cancelled and failed scans are not persisted;
- score derives only from supported, evidence-backed findings and reports partial coverage honestly;
- unsupported categories remain explicit `unavailable` states and do not reduce the score;
- no destructive action or Administrator privilege is used during scan;
- frontend, Rust, Windows inventory smoke test, and native Windows build are green in CI.

---

## P4 — Smart Cleaner + Restore Center foundation

Goal: first write/destructive subsystem, built safely.

Smart Cleaner V1:

- [x] Windows temp candidates
- [x] application temp providers with explicit rules
- [x] browser cache providers with explicit rules
- [~] recycle bin as a separate opt-in item — modeled and opt-in, native size/count provider still gated
- [x] scan-only preview
- [x] cleanup plan
- [x] execution result

Restore foundation:

- [x] operation log — append-only local JSONL audit under LOCALAPPDATA
- [x] before-state metadata — native plan ID, requested bytes/files, provider totals, and per-item scan metadata used for execution revalidation; personal paths are not persisted in history
- [x] rollback capability flags — cache/temp deletion is explicitly marked not restorable
- [x] rollback UI — Restore Center shows operation history and honest rollback availability

Important:

Not every deleted cache file is restorable. The app must be honest about that. "Restore Center" records operations and restores only operations for which a safe rollback exists.

Acceptance:

P4A preview gate:
- [x] exact byte estimate before cleanup;
- [x] user can inspect categories/providers;
- [x] no arbitrary filesystem root can be supplied by the frontend;
- [x] reparse points/symlinks are skipped;
- [x] provider errors are isolated into warnings;
- [x] native cleanup plan is retained by plan ID;
- [x] no delete command exists in P4A;
- [x] Windows CI executes the real built-in scanner and native build successfully;
- [x] user verified preview totals and provider breakdown on a real Windows machine before P4B.

P4B execution gate:
- [x] every planned path is canonicalized and revalidated immediately before deletion;
- [x] reparse points, changed files, out-of-root paths, unsupported providers, and stale-rule failures are skipped;
- [x] errors do not abort unrelated cleanup items;
- [x] audit storage is preflighted before deletion and the completed operation record is appended locally;
- [x] execution result shows requested/deleted/failed files and bytes accurately;
- [x] restore capability is labeled honestly as "Not restorable" for cache/temp deletion;
- [x] disposable Windows CI tests prove deletion of an unchanged planned file and rejection of a candidate changed after scan;
- [x] frontend, Rust, Clippy, Windows tests, and native Windows build are green.

---

## P5 — Startup Manager

Goal: safe startup control.

Sources:

- [x] supported Run/RunOnce registry locations
- [x] startup folders
- [x] selected scheduled tasks when they clearly represent startup behavior

Capabilities:

- [x] inventory
- [x] enable/disable where safe
- [x] publisher/source display — publisher remains unavailable when Windows does not provide trustworthy evidence
- [x] command display
- [x] rollback record
- [x] search/filter

Acceptance:

- [x] supported current-user Run/RunOnce entries preserve rollback state in the PC Manager backup namespace;
- [x] current-user Startup folder items use a PC Manager-owned disabled store and restore to their recorded Startup folder;
- [x] machine-wide/elevated entries remain read-only instead of elevating the whole desktop app;
- [x] scheduled startup/logon tasks are discovered; unsupported permission changes fail closed;
- [x] disabling startup never invokes an uninstaller;
- [x] every attempted mutable change produces an operation record;
- [x] frontend, Rust tests, Clippy, and native Windows build pass in CI;
- [~] real Windows verification: inventory has been proven on the user's Windows machine; the disposable enable-disable-enable round trip is intentionally deferred by user direction and remains an open P5 verification item.

Sequencing exception recorded 2026-10-06: P6 implementation may proceed while this P5 round-trip remains open. P5 must not be represented as fully accepted until that round-trip is completed.

---

## P6 — Apps, Duplicates, Storage

### Uninstaller

- [x] enumerate installed programs
- [x] search/filter
- [x] open standard uninstall flow
- [x] refresh after uninstall — explicit refresh after the external uninstaller returns
- [x] no forced silent uninstall in V1

### Duplicate Finder

- [x] folder selection — native Windows folder picker with session-scoped root IDs
- [x] exclusions
- [x] size grouping
- [x] partial hashing
- [x] full hashing
- [x] preview
- [x] user-selected deletion only

### Storage Analyzer

- [x] tree aggregation
- [x] largest files
- [x] file-type grouping
- [x] exclusions
- [x] cancellation

Acceptance:

- [x] symlink/reparse-point loops are skipped by the shared filesystem walker;
- [x] inaccessible paths become bounded warnings instead of crashing scans;
- [x] duplicates are declared only after exact-size, partial-hash, and full-hash verification;
- [x] duplicate deletion uses opaque scan IDs, preserves at least one verified copy, revalidates path/metadata/full hash immediately before deletion, and records an audit result;
- [x] Apps rediscover the selected opaque app ID before launch and never accept a frontend-supplied command line;
- [x] silent uninstall flags are rejected and MSI uninstall uses the standard interactive flow;
- [x] frontend lint/typecheck/tests/build/format, Rust fmt/check/tests/Clippy, Windows provider tests, and native Windows build are green in CI;
- [ ] representative P6 behavior is verified manually on a real Windows machine — deferred by explicit user instruction.

---

## P7 — System Monitor

Goal: low-overhead useful telemetry.

- [x] CPU
- [x] RAM
- [x] disk activity where reliable
- [x] network
- [x] process top consumers
- [x] GPU where reliable
- [x] sensor capability detection

Acceptance:

- [x] unsupported sensors show "Unavailable" instead of fabricated values;
- [x] sampling interval is configurable with conservative 2s/5s/10s choices;
- [x] monitor can be paused and requests no new native samples while paused;
- [x] every native snapshot records provider sample duration and the UI surfaces it;
- [x] Windows CI executes the real monitor provider and native build successfully;
- [x] frontend lint/typecheck/tests/build/format and Rust fmt/check/tests/Clippy are green before merge;
- [ ] representative idle-overhead behavior is measured manually on a real Windows machine — deferred by explicit user instruction.

---

## P8 — App Manager integration

Goal: connect PC Manager to the existing administration platform through the approved outbound-only Desktop Agent Gateway.

Client capabilities:

- [x] device identity — persistent Windows CNG P-256 key; only the public JWK leaves the device
- [x] registration — idempotent `pc-manager/windows/desktop-native` registration
- [x] approval state — pending/approved/blocked
- [x] heartbeat — one-time signed challenge with server-directed interval
- [x] online/offline semantics — heartbeat presence is separate from local PC Manager availability
- [x] app version
- [x] release channel
- [x] entitlement/license state
- [x] update policy
- [x] typed remote command envelope

Remote command allow-list initially:

- [x] CHECK_UPDATE
- [x] RUN_HEALTH_SCAN
- [x] REFRESH_DEVICE_STATUS
- [x] DISABLE_LICENSE

Explicitly prohibited:

- arbitrary shell;
- arbitrary PowerShell supplied by the server;
- arbitrary process execution;
- arbitrary registry mutation;
- arbitrary file download-and-run.

Acceptance:

- [x] offline/not-configured mode leaves local PC Manager maintenance usable;
- [x] retries use bounded backoff;
- [x] the device private key remains in Windows CNG instead of plaintext application storage;
- [x] server commands deserialize into and dispatch through a four-command allow-list only;
- [x] health-scan command returns privacy-bounded aggregate evidence instead of paths/file contents;
- [x] Application Management exposes a separate authenticated admin surface while the native transport is outbound-only;
- [x] frontend/Rust/native Windows CI are green for the final P8 client head;
- [ ] Production gateway migration/deployment and a real signed PC Manager handshake are verified — deferred until the approved HTTPS Application Management origin is deployed/configured.

---

## P9 — Production packaging and update system

Goal: reliable release pipeline.

- [x] preview build workflow — unsigned NSIS artifact, explicitly labeled preview-only
- [x] production workflow — manual signed release from `main`
- [x] production confirmation input — exact `DEPLOY_PRODUCTION`
- [x] Windows installer — NSIS bundle target
- [x] checksums — SHA-256 manifest plus per-artifact metadata digest
- [x] signing integration hooks — production fails closed without the configured PFX credentials
- [x] release notes — generated from the release commit history
- [x] updater metadata — `pc-manager.update/v1`
- [x] stable/beta/dev policy — enforced by `pc-updater` policy helpers
- [x] rollback/failed-update strategy — documented in `docs/RELEASE_POLICY.md`

Desired production trigger pattern:

```powershell
gh workflow run production-release.yml --ref main -f confirm=DEPLOY_PRODUCTION
```

Acceptance:

- [ ] a clean Windows machine can install the produced signed installer — manual gate;
- [ ] upgrade preserves local state and the CNG App Manager identity — manual gate;
- [x] downgrade/rollback policy is documented and normal update policy blocks automatic downgrade;
- [x] unsigned test builds are clearly distinguishable from production;
- [x] production packaging refuses to proceed without signing credentials;
- [x] automated frontend/Rust/native packaging checks are green before merge.

---

## P10 — Software Updater

Sequencing exception recorded 2026-10-07: implementation may proceed while P9's clean-machine install and upgrade/state-preservation manual gates remain open. P9 must not be represented as fully accepted until those gates are proven.

Goals:

- [~] detect supported application updates;
- [~] verify publisher/source — official WinGet source trust is verified in V1; publisher evidence must remain explicit and may be unavailable until structured verification exists;
- [x] prefer trusted vendor/package-manager mechanisms — WinGet is the V1 provider;
- [x] never download executables from unverified URLs;
- [ ] real Windows representative update launch is manually verified.

V1 safety boundary:

- no arbitrary URL download;
- no Update All;
- no frontend-supplied command line or WinGet flags;
- exact package ID from a native opaque scan plan only;
- revalidate the candidate immediately before launch;
- official trusted `winget` source only;
- request interactive installer mode and never bypass installer hash checks.

---

## P11 — Driver Center

Sequencing exception recorded 2026-10-07: implementation may proceed while the documented P9/P10 manual gates remain open. Those earlier phases must not be represented as fully accepted until their manual gates are proven.

V1 capabilities:

- [x] enumerate installed signed-driver inventory locally;
- [x] discover driver-class updates through Windows Update Agent without installing them;
- [x] use opaque local identifiers rather than exposing raw hardware IDs as control tokens;
- [x] explain available update evidence and provider/source state;
- [x] old driver age alone never creates a recommendation;
- [x] no mass update / Update All action;
- [x] no arbitrary driver URL, download-and-run, force-install, downgrade, or unsigned-driver path;
- [x] hand installation control to the standard Windows Optional Updates surface;
- [x] bounded provider results and warnings;
- [ ] representative real-Windows inventory and Optional Updates handoff are manually verified.

Restore protection rule:

P11 V1 performs no direct driver mutation, so it does not fabricate a restore point or rollback promise. If a later phase performs direct driver installation, restore-point/rollback feasibility becomes a mandatory gate before that mutation is enabled.

---

## P12 — Advanced Performance Optimizer

Sequencing exception recorded 2026-10-07: P12 implementation may proceed while earlier manual verification gates remain tracked as open. Those earlier phases must not be represented as fully accepted until their manual gates are proven.

V1 capabilities:

- [~] boot-impact analysis — P12 correlates enabled startup/logon sources with repeatedly measured resource consumers; it does not fabricate boot-duration timing that Windows did not provide;
- [x] persistent resource consumers — three short foreground samples with repeated-visibility plus CPU/RAM thresholds;
- [~] optional application sleep policies — advisory foundation only; no universal process suspension is enabled because arbitrary desktop-app suspension can break apps or lose work;
- [x] explainable scheduled-task recommendations — only when a P5 startup/logon task maps to repeated process evidence;
- [x] explainable service recommendations — read-only, bounded automatic-running service evidence; Windows-system-path services are excluded from recommendations;
- [x] no direct service/task disable path;
- [x] no process kill/suspend/freeze path;
- [x] supported startup changes remain owned by P5 Startup Manager and its rollback/revalidation rules;
- [x] no generic registry, power-plan, pagefile, Defender, Windows Update, indexing, networking, or visual-effects tuning;
- [ ] representative real-Windows gate verifies a known high-resource app is surfaced with evidence and an idle/normal machine does not generate fake optimization warnings.

V1 thresholds are evidence gates, not claims of defect: a process must appear in multiple samples and cross a documented CPU or private-memory threshold before it is surfaced.

Avoid "disable everything" optimization.

---

## P13 — Product release-readiness evidence preflight

Implementation may proceed without fabricating completion of P5–P12 manual gates. The user authorized continuation on 2026-10-08; this exception does **not** authorize a production release.

- [x] maintain nine mandatory real-Windows acceptance gates in `docs/release-acceptance.json`, all initially pending;
- [x] fail-closed schema validator for missing, duplicated, invalid, or unproven gates;
- [x] automated negative tests for pending and forged acceptance;
- [x] routine CI validates the manifest without falsely requiring outstanding manual checks to pass;
- [x] production release workflow requires `--require-ready` before packaging;
- [x] investigate and resolve the two Critical and one Moderate npm advisories reported on 2026-10-08: upgrade the Vitest development toolchain to `^5.0.3` and regenerate the lockfile; npm audit reported zero vulnerabilities after the targeted update. CI now enforces `npm audit --audit-level=critical`;
- [ ] design/version/authenticate the currently skeletal privileged Windows Service IPC before adding elevated operations — P15B1 read-only admission, P15B2 SID extraction, and P15B3 token impersonation-level/session evidence are foundations only; transport still inactive;
- [ ] implement and test safe SQLite migrations for durable local state where warranted;
- [ ] obtain authentic signed-install, upgrade, gateway, device and efficacy evidence for every mandatory gate.

P13 V1 acceptance:

- [ ] CI and unsigned Windows preview are green on the final P13 commit;
- [ ] `--validate` succeeds for honest pending gates;
- [ ] `--require-ready` refuses a manifest with pending/forged evidence;
- [ ] production signing credentials, exact dispatch confirmation, protected environment and checksums remain mandatory.

---

## P14 — Dependency security baseline

Scope: remove known npm security vulnerabilities without a broad refactor or weakening release gates.

- [x] classify the CI npm audit warnings: `tinypool` (two Critical advisories) and `@vitest/mocker` (one Moderate advisory), reached via the Vitest development/test toolchain;
- [x] upgrade only Vitest and its required resolved dependencies (`vitest: ^5.0.3`) using npm's lockfile resolver, without `npm audit fix --force`;
- [x] observe `npm audit` return **0 vulnerabilities** on the regenerated dependency lockfile in the remediation CI job;
- [x] add a blocking critical-severity npm audit to normal frontend CI;
- [x] remove the one-time CI lockfile-write job and its extra `contents: write` permissions;
- [ ] confirm final frontend tests, Rust/Windows builds and unsigned preview on the final branch head before merge.

No elevated Windows mutation, new remote command, telemetry, production release or change to the nine P13 manual acceptance gates is authorized by this phase.

---

## Managed offline launcher — post-P13 safety hardening

- [x] APP Manager's icon uses `pcmanager://open` without a privileged browser API.
- [x] NSIS Preview registers an owned, argument-free per-user protocol (PR #43).
- [x] Windows single-instance mutex + activation event is implemented and passed exact-HEAD Windows CI and unsigned Preview in merged PR #44; real-browser user acceptance remains pending.
- [ ] Chrome/Edge → installed PC Manager repeat-click and minimize/restore behavior verified on a real user Windows machine.
- [ ] Signed Stable, nine original P5–P12 acceptance gates and Production authorization remain pending.

Uses existing Windows kernel primitives under the `windows-host` dependency
budget, no new network provider or privileged surface.

---

## Working cadence

For each phase:

1. plan;
2. implement;
3. test;
4. inspect failures;
5. fix;
6. update docs;
7. merge;
8. create preview artifact where relevant;
9. proceed only after the acceptance gate passes.

P12 implementation is present, but its manual Windows acceptance remains pending. P13 is the release-readiness hardening phase; no production release is authorized until all evidence and signing gates are satisfied.

## Offline-first managed-mode vertical slice — governance-aligned product work

Source: active Hub `control/development-runtime-policy.v1.json` under
Constitution 1.3.0; Hub PR #141/#147 remain candidates and are not adopted.

Goal: a Windows user runs PC Manager locally with no gateway account, network,
approval or cloud runtime; the user may **explicitly opt in** to outbound
App Manager synchronization later.

- [~] Settings offers Central management OFF (default) / ON using local
  persisted preference; OFF stops new UI-scheduled synchronization.
- [~] Typed native local capabilities remain reachable regardless of mode.
- [~] Frontend unit tests cover fresh install, explicit ON/OFF persistence,
  and storage-denied fail-closed behavior; exact-HEAD CI required.
- [ ] Real installed Windows device: disconnect Internet; open Overview,
  run Health Check and Smart Clean scan preview; restart and verify
  theme/sidebar/managed-mode preference persistence and error recovery.
- [ ] With managed OFF, independently observe no App Manager outbound
  traffic. Toggle ON with a configured safe test gateway, verify status,
  toggle OFF and confirm no subsequent scheduled sync. In-flight calls
  may complete before OFF; never claim instantaneous network cancellation.
- [ ] Browser `pcmanager://open` handoff, signed installer/state upgrade
  and nine mandatory production gates remain separately UNVERIFIED.

Completion predicate: actual offline-local user journeys and persistence
evidence (exact build + expected/actual + tester), not source/UI or CI alone.
No new hub policy, remote runtime, package dependency or Product PASS.

## P16 — native offline Windows acceptance automation (pilot)

- [~] Install an unsigned NSIS build on disposable Windows, block outbound
  traffic for installed Tauri + WebView2 executables, then invoke REAL UI
  Automation for native launch, Health Check, Smart Clean preview, managed
  OFF/ON/OFF and local report/mode state across a process restart.
- [~] Produce exact-revision machine-readable evidence even on failures.
  Missing firewall, UI Automation or WebView2 is a FAIL, not a fake PASS.
- [ ] Complete exact-HEAD CI and preview verification.
- [ ] True full-network-disconnected end-user Windows acceptance, signed
  install/upgrade, browser handoff, verified gateway traffic and nine
  original real-Windows release gates remain pending.

Firewall rules are per binary and restored; no cleanup action is executed.

### Smart Clean freeze-risk correction (in PR #47)

- Actual Windows NSIS UIA run #38989639836: native launch PASS, blocked
  outbound app+WebView2 PASS, Health Check PASS; Smart Clean preview NOT
  OBSERVED after 120s. Root cause has **not** been proven from UIA alone.
- Native `scan_cleanup_candidates` was a synchronous Tauri command calling
  potentially long recursive filesystem enumeration. The command is now
  async with `spawn_blocking` for enumeration; cleanup-plan state is locked
  only after the preview finishes. No deletion path or scan roots changed.
- UIA timeouts now record allow-listed Smart Clean state markers (scanning,
  error, idle, preview), without leaking raw filenames.
- [ ] Confirm exact-HEAD Windows live UI behavior and all CI; keep blocked
  if the scan remains too slow. Do NOT treat code restructuring as acceptance.

## P17 — offline preference durability and explicit recovery

Work package: native local WebView2 preference failure handling and observed
restart readback. Builds on verified P16; no privilege or remote runtime.

- [x] Fail visibly and keep the current appearance in memory if localStorage
  cannot write or read back the selected theme or sidebar value.
- [x] Require readback equality for opt-in managed ON/OFF, retaining OFF by
  default on missing/invalid/inaccessible storage.
- [x] Give appearance controls stable accessibility names reflecting their
  selected state, without changing behavior or network privileges.
- [ ] On the exact PR head, Windows unsigned NSIS native UIA must observe:
  Health + non-destructive Smart Clean preview, dark theme, compact sidebar,
  managed ON surviving a process restart; managed OFF surviving another
  restart, all under scoped outbound-deny firewall rules.
- [ ] Signed upgrade, whole-PC Internet disconnected acceptance, verified
  real gateway traffic, personal Windows acceptance and original nine
  P5-P12 gates stay separate and pending. P17 cannot mark these PASS.

## P18 — unsigned NSIS same-version repair persistence pilot

Incremental, non-destructive test-only critical path after P17:
- [x] After a true unsigned NSIS install, run the P17 native UIA test under
  scoped outbound deny and close the process.
- [x] Invoke the **same** unsigned NSIS installer silently a second time on
  the same disposable Windows runner; verify executable and owned HKCU
  argument-free protocol handler are still correct.
- [x] Reopen the actual native Tauri/WebView2 app with outbound deny and
  require retained local theme, sidebar, managed OFF and saved Health report.
  Generate a separate exact-revision PASS/FAIL JSON artifact.
- [ ] Exact-HEAD Preview and all CI gates must pass before merge.
- [ ] This is *not* a version-to-version upgrade, signed installer migration,
  CNG identity migration or user-device acceptance. The nine P5–P12 manual
  release gates are unaffected. No data deletion or privileged IPC.

## P19 — disposable unsigned version-transition pilot

Work package: evidence of state retention across **different NSIS bundle version
labels** on the same disposable Windows runner. Baseline is the actual Preview
version; candidate is built from the **same checked-out source** with a temporary
Tauri CLI `--config` override bumping the patch version. The candidate never
enters distributable Preview artifacts and the repository version is unchanged.

- [x] Enforce distinct, increasing semantic bundle version labels and distinct
  installer SHA256 checksums before replacing the installed package.
- [x] Build the version-only candidate in CI; after P18 same-version repair,
  invoke the higher-version installer with the app closed. Verify the owned
  argument-free HKCU handler and executable still exist, then launch REAL UIA
  to read managed OFF, dark theme, compact sidebar and persisted Health report.
- [x] Emit SHA-bound independent version-transition evidence with the version
  pair, both installer hashes, verification mode and explicit signed/rollback
  disclaimers; do not distribute the synthetic candidate.
- [ ] Exact-HEAD CI and Windows Preview evidence must PASS before merge.
- [ ] A real earlier source release upgraded to a newer source release, signed
  installation/upgrade and signer trust, installer failure mid-transaction,
  automatic rollback, personal Windows airgap and P5–P12 release gates remain
  UNVERIFIED. No user files are cleaned and no privileged IPC is enabled.

## P20 — fail-closed pre-install candidate validation, disposable Windows

Following real native P16–P19 Preview acceptance on unchanged HEAD, P20 hardens
the **CI upgrade pilot** before it is allowed to invoke the unsigned candidate
installer. No Production update manager or signature trust is introduced.

- [x] Add a read-only, explicit `assert-nsis-upgrade-preflight.ps1` that
  rejects missing files, malformed bundle names, version mismatch, non-increasing
  versions, missing/wrong pinned SHA256, and byte-identical packages. Positive
  case must use the stage-pinned exact candidate SHA256.
- [x] In the disposable Windows runner, execute seven genuine negative cases
  while the old version remains installed; after each, compare installed
  executable SHA256 and exact owned HKCU URI handler to the original. After all
  rejections, reopen the *real* Tauri/WebView2 app under scoped outbound block
  and require managed OFF, dark theme, compact sidebar and saved Health report.
- [x] Run preflight again immediately before the one permitted candidate install.
  Export independent SHA-bound failure/success evidence even if the check fails.
- [x] Include both PowerShell scripts in the early parser gate.
- [ ] Exact-HEAD CI, Constitution, and Windows NSIS runtime evidence must all
  succeed before merge; no extrapolation from synthetic version changes.
- [ ] Scope limitations: tests invoke **no installer for rejected candidates**;
  this is not an actual failed-installer rollback, Authenticode signature
  verification, tamper-proof distribution, TOCTOU elimination, two-source-version
  upgrade, human airgap acceptance or signed Production readiness. P5–P12
  release-acceptance statuses remain pending. No privileged IPC or cleanup.


## P21 — SHA-pinned NSIS candidate handoff with a write-denying file handle

Product-critical next action after merged P20: fail closed if the unsigned
development upgrade candidate is currently writable or changes across final
preflight and the NSIS launch boundary, without granting updater authority.

- [x] Keep an open, read-only `FileShare.Read` handle from final P20 version
  and SHA256 preflight through the child installer process exit; recompute
  SHA256 directly from the locked handle before invoking NSIS.
- [x] Refuse a candidate that cannot be locked. Run a genuine negative case
  with an existing writable handle on a disposable candidate copy and verify
  the installed executable SHA256 and owned HKCU handler remain unchanged.
- [x] Replace the positive P19 NSIS execution path with the only locked
  handoff; preserve existing P20 negative cases and native state readback.
- [ ] Parse all scripts and require Constitution, CI, unsigned NSIS Preview,
  real Windows negative and positive tests on **the same exact PR SHA** before
  a merge. Observe native NSIS startup under the held read handle; do not
  infer compatibility from syntax alone.
- [ ] Full filesystem/path/symlink TOCTOU resistance, publisher/signature
  trust, signed releases, failure-mid-install rollback, two-source-version
  migration and real user Windows acceptance are **UNVERIFIED**. This is
  a bounded, unsigned developer-runner test, not a Production updater.

Do not alter privileged IPC, cleanup execution, App Manager authority,
Production status or the nine mandatory pending P5–P12 release gates.


## P22 — genuine two-source unsigned NSIS upgrade on disposable Windows

Product goal: move from a real earlier compiled application's local state to
a **different application source revision**, not merely a version-label bump
of the same tree (P19–P21). Preserve offline-first, zero Production authority.

- [x] Pin the older-source baseline to real main
  `83645a56987666144020dbdc85b3eec12815d003`, fetch it into a distinct
  detached worktree, verify Git HEAD, and compile its actual 0.1.0 Windows NSIS
  with the normal Tauri workflow. Keep its installer in runner TEMP only.
- [x] Require nonempty changed app source paths on the candidate HEAD, with a
  small, meaningful Settings clarification about local user-profile ownership.
  Build the candidate from **exact PR HEAD**, not the synthesized PR merge ref.
- [x] Use a temporary **unsigned development-only** 0.1.1 Tauri version
  override for the newer source while the repo declares 0.1.0; artifact
  labels explain this distinction. This is not a Production version bump.
- [x] Pin and compare the built baseline/candidate executable SHA256 values;
  ensure the installed baseline and installed upgraded candidate match their
  respective compiled bytes, and are different from each other.
- [x] Require the newer source's Settings marker in actual post-upgrade WebView2
  UIA. Preserve local Health report, managed OFF, dark theme and compact
  sidebar under app/WebView2-scoped outbound deny. Retain P20 rejection cases,
  P21 locked candidate handoff and HKCU exact handler validation.
- [x] Emit SHA-bound `offline-two-source-build-evidence.json` and separate
  native upgrade/readback JSON in the **unsigned Preview CI** artifacts.
- [ ] Before merge: Constitution 1.3.0, frontend/Rust/Windows native checks,
  PowerShell parser, NSIS installer lifecycle, real native UIA, SHA and binary
  provenance evidence must all PASS for the exact candidate SHA. Review gate
  and GitHub main readback are required.

**Limitations:** no signed Authenticode trust, signer continuity, actual
mid-install failure/rollback, production two-release migration contract, full
device airgap, user Windows acceptance or P5–P12 release-gate promotion.
The baseline is immutable historical source and the candidate is an unsigned
CI development artifact; not an approved user update. No elevated service,
remote shell, privileged IPC or destructive maintenance is introduced.
