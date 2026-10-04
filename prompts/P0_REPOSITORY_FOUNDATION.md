# P0 EXECUTION PROMPT — Repository Foundation

You are implementing **P0 — Repository foundation** for `pc-manager-desktop`.

Before changing code, read:

1. `/AGENTS.md`
2. `/docs/MASTER_BUILD_PROMPT.md`
3. `/docs/IMPLEMENTATION_PLAN.md`
4. `/docs/ARCHITECTURE.md`

Do not redesign the architecture unless you find a concrete blocker. If a blocker exists, document it before changing direction.

---

## Objective

Create a clean, reproducible Windows-first monorepo foundation for PC Manager Desktop.

At the end of P0, a developer must be able to clone the repository, install dependencies, run the desktop app in development, build it, and run the project checks.

Do **not** implement Health Check, Cleaner, Startup Manager, monitoring, App Manager networking, or privileged Windows Service behavior yet.

P0 is infrastructure only.

---

## Required technology

Use:

- React
- TypeScript
- Vite
- Tauri
- Rust workspace
- GitHub Actions

Target:

- Windows 10/11 x64

Architecture target:

```text
pc-manager-desktop/
├─ apps/
│  └─ desktop/
│     ├─ src/
│     ├─ src-tauri/
│     ├─ package.json
│     ├─ tsconfig.json
│     └─ vite.config.*
├─ crates/
│  ├─ pc-core/
│  ├─ pc-windows/
│  ├─ pc-monitor/
│  ├─ pc-updater/
│  └─ app-manager-client/
├─ services/
│  └─ windows-service/
├─ contracts/
├─ installer/
├─ scripts/
├─ tests/
├─ prompts/
├─ docs/
├─ .github/
│  └─ workflows/
├─ Cargo.toml
├─ package.json
├─ .editorconfig
├─ .gitignore
└─ README.md
```

Empty directories do not need placeholder files unless useful.

---

## Step 1 — Inspect before creating

Inspect the current repository first.

Do not overwrite planning documents.

Confirm:

- current default branch;
- existing files;
- whether package manifests already exist;
- whether any previous P0 work exists.

Reuse valid existing work.

---

## Step 2 — Root workspace

Create a root workspace that provides simple commands for developers.

The root should make these concepts easy:

```text
install
dev
build
lint
typecheck
test
format
format:check
```

Use a package-manager workspace approach that is compatible with the chosen React/Tauri setup.

Avoid unnecessary monorepo frameworks in P0.

Do not introduce Nx, Turborepo, Bazel, or similar tooling unless there is a demonstrated need.

---

## Step 3 — Rust workspace

Create a Rust workspace with these members or equivalent stable paths:

- `apps/desktop/src-tauri`
- `crates/pc-core`
- `crates/pc-windows`
- `crates/pc-monitor`
- `crates/pc-updater`
- `crates/app-manager-client`
- `services/windows-service`

For P0, each library/service may contain only a minimal compile-safe skeleton.

Important:

- no fake implementation;
- no unused elaborate abstractions;
- no privileged behavior yet;
- no network calls yet.

Each crate should clearly state its purpose in its crate-level docs or README.

---

## Step 4 — Desktop application shell

Create the Tauri + React + TypeScript application under:

```text
apps/desktop
```

P0 UI should be deliberately minimal.

Required screen:

```text
PC Manager Desktop

Foundation status
✓ React UI
✓ Tauri shell
✓ Rust workspace

Phase: P0
```

Do not build a fake CCleaner UI yet.

The purpose is to prove the native shell and frontend/backend bridge work.

Add one harmless Tauri command such as:

```text
get_app_info()
```

It may return:

- appId: `pc-manager`
- platform: `windows`
- deviceType: `desktop-native`
- phase: `P0`

The React UI should call that command and render the result.

This proves the typed desktop bridge before real system features are introduced.

---

## Step 5 — Code quality

Configure:

### TypeScript

- strict mode;
- no obvious `any` escape hatches in new code;
- linting;
- formatting.

### Rust

Required checks:

```bash
cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

If a platform-specific crate cannot compile on non-Windows runners, isolate it correctly with conditional compilation rather than deleting the check entirely.

Do not suppress Clippy warnings globally.

---

## Step 6 — Git ignore and editor settings

Add a correct `.gitignore` covering at least:

- Node dependencies;
- Vite output;
- Rust target output;
- Tauri build artifacts;
- local env files;
- IDE noise;
- OS noise.

Do not ignore lockfiles that are required for reproducible builds.

Add `.editorconfig` with sensible defaults.

---

## Step 7 — CI

Create GitHub Actions CI.

At minimum:

### Frontend job

- install dependencies from lockfile;
- lint;
- typecheck;
- test if tests exist;
- build frontend.

### Rust job

- format check;
- cargo check;
- cargo test;
- clippy.

### Windows/Tauri build job

Run on a Windows runner and prove that the native app can build.

Do not publish a production release in P0.

Do not require code-signing secrets in P0.

CI must fail when code quality checks fail.

---

## Step 8 — Basic tests

Add small tests with real value.

Examples:

- Rust test for application metadata model;
- frontend test for the P0 status component or app-info rendering.

Do not create dozens of trivial snapshot tests merely to inflate coverage.

---

## Step 9 — Documentation

Update `README.md` with exact development commands.

Document:

- prerequisites;
- dependency installation;
- dev command;
- build command;
- test/check command;
- current phase;
- Windows-first scope.

Do not claim production readiness.

---

## Step 10 — P0 acceptance gate

P0 is complete only when all are true:

- [ ] root workspace exists;
- [ ] React + TypeScript + Vite app exists;
- [ ] Tauri shell exists;
- [ ] Rust workspace exists;
- [ ] all planned crates exist and compile;
- [ ] Windows Service project exists as a non-privileged skeleton only;
- [ ] frontend can invoke one harmless Tauri command;
- [ ] lint passes;
- [ ] typecheck passes;
- [ ] Rust formatting passes;
- [ ] Rust tests pass;
- [ ] Rust Clippy passes;
- [ ] frontend build passes;
- [ ] Windows native/Tauri CI build passes;
- [ ] README development instructions are accurate;
- [ ] no secrets are committed.

Only then update the P0 checklist in `docs/IMPLEMENTATION_PLAN.md`.

---

## Branch and PR policy

Use a dedicated branch such as:

```text
feat/p0-repository-foundation
```

Keep commits coherent.

Open a PR into `main`.

PR description must include:

```text
Phase: P0

Completed:
- ...

Validation:
- npm/pnpm ...
- cargo ...

CI:
- ...

Known limitations:
- ...

Next:
- P1 Application shell and design system
```

Do not merge with red CI.

If CI fails, inspect the actual failure, fix it, rerun checks, and continue until P0 acceptance criteria pass or a genuine external blocker is reached.

---

## Stop conditions

Stop and report rather than bypassing when:

- the repository permissions prevent required writes;
- Windows build failure requires unavailable signing credentials even though signing should not be needed in P0;
- a Tauri/Rust dependency conflict cannot be resolved without changing the agreed stack;
- CI cannot run because GitHub Actions are disabled;
- a requested change would introduce arbitrary elevated command execution.

---

## Final report

When P0 is ready, output only a concise implementation report in this structure:

```text
P0 — Repository Foundation

Status:
Branch:
PR:

Completed:
- ...

Checks:
- ...

CI:
- ...

Known limitations:
- ...

Next:
P1 — Application shell and design system
```

Do not claim PASS unless the evidence supports it.
