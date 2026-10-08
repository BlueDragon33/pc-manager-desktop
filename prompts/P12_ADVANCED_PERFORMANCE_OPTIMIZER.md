# P12 — ADVANCED PERFORMANCE OPTIMIZER

## Objective

Build an evidence-first optimizer that explains measurable performance cost without using scareware or broad Windows mutations.

P12 must reuse the trusted P5 Startup Manager and P7 Monitor foundations instead of inventing a second mutation path.

## Non-negotiable safety rules

1. Never implement “disable everything”, “boost all”, or a one-click service shutdown.
2. A single CPU spike is not evidence of a persistent problem.
3. Do not call an application bad merely because it starts with Windows.
4. Do not disable services or scheduled tasks directly from Performance Optimizer V1.
5. Do not kill, suspend, freeze, or terminate arbitrary user processes.
6. Do not implement a generic registry-tuning optimizer.
7. Do not change power plans, pagefile settings, Defender, Windows Update, indexing, SysMain, networking, or visual effects without a separate evidence and rollback design.
8. Any supported startup mutation must remain owned by Startup Manager, including its revalidation and rollback records.
9. Windows/system-path services must not be recommended for disabling from this phase.
10. Do not fabricate boot duration, startup impact, publisher evidence, or long-term resource usage.

## V1 evidence model

Collect several short foreground samples of:
- total CPU;
- total memory pressure;
- top processes with CPU and private memory.

Correlate repeated process evidence with:
- enabled startup entries;
- startup/logon scheduled tasks already exposed by P5;
- running automatic services from a bounded read-only Windows inventory.

A candidate is “persistent” only when it appears in multiple samples and crosses a documented CPU or memory threshold.

## Findings

Supported finding kinds:
- persistent process;
- startup evidence;
- scheduled-task evidence;
- service review evidence.

Every finding must include:
- what was observed;
- why it is being shown;
- concrete evidence;
- risk;
- a restrained recommendation;
- a safe next action.

## App sleep policy

P12 V1 is advisory only.

Do not pretend Windows has one safe universal sleep API for arbitrary desktop applications. Recommend the application's own background/startup settings or the existing Startup Manager. A future direct sleep mechanism requires a separate design with app compatibility, unsaved-work protection, rollback, and user-visible policy.

## Service/task policy

Service/task recommendations are review-only in P12 V1.

A non-system-path automatic service may be surfaced only when its running process matches a repeatedly measured resource consumer. No Stop-Service, Set-Service, sc.exe mutation, task disable, arbitrary PowerShell, or elevation path is allowed.

## Acceptance

- frontend lint/typecheck/tests/build/format pass;
- Rust fmt/check/tests/Clippy pass;
- Windows native provider/build pass;
- preview NSIS package passes;
- unit tests prove one spike is not classified persistent;
- unit tests prove system-path services are not recommended;
- no process termination/suspension or service/task mutation primitive exists;
- real-Windows manual gate verifies that repeated high-resource activity produces explainable evidence and that an idle machine does not create fake warnings.
