# P9 EXECUTION PROMPT — Production Packaging, Update Metadata, and Release Hardening

Read `/AGENTS.md`, `/docs/MASTER_BUILD_PROMPT.md`, `/docs/IMPLEMENTATION_PLAN.md`, and `/docs/ARCHITECTURE.md` before changes.

## Goal

Create a reproducible Windows release pipeline that keeps preview and production separate, produces an installer plus checksums/update metadata, and never treats an unsigned artifact as Production.

## Release channels

- `dev`: CI/preview only; may be unsigned and must be labelled as such.
- `beta`: manually released production-path artifact for opt-in testing.
- `stable`: manually released production-path artifact.

Channel policy must be represented in `pc-updater`; do not infer channels from filenames.

## Preview build

Create a Windows workflow that:

- runs only by explicit workflow dispatch or pull-request/reusable verification as configured;
- runs normal frontend/Rust checks before packaging;
- builds an NSIS installer;
- permits unsigned packaging only by explicit preview environment flag;
- renames/labels artifacts with `UNSIGNED-PREVIEW`;
- computes SHA-256 checksums;
- uploads artifacts with finite retention;
- never creates or updates a Production GitHub Release.

## Production release

Create a manual workflow with exact confirmation:

```text
confirm=DEPLOY_PRODUCTION
```

Requirements:

- only run from `main`;
- run checks again before packaging;
- require Windows signing secrets at runtime;
- never log certificate/private-key material;
- package an NSIS installer;
- Authenticode signing is invoked through a repository-owned signing hook;
- verify the resulting signature before publication;
- calculate SHA-256 after signing;
- generate machine-readable update metadata from the signed artifact;
- publish release notes, installer, checksum, and updater metadata together;
- no automatic promotion from preview.

If signing material is absent, Production must fail closed.

## Signing hook

The repository may contain only a signing script/integration hook.

Secrets such as a PFX, password, hardware-token credential, Azure Trusted Signing credential, or private updater key must never be committed.

A preview build may bypass signing only with an explicit `PC_MANAGER_ALLOW_UNSIGNED_PREVIEW=1` flag.

## Update metadata

Metadata must include at least:

- version;
- channel;
- mandatory flag;
- SHA-256;
- artifact URL;
- release notes;
- published timestamp;
- platform/architecture.

The update client must reject malformed channels/checksums and must verify the downloaded artifact checksum before handoff.

## Rollback policy

Document:

- application state lives outside the install directory and must survive upgrades;
- failed installation does not authorize deleting local state;
- automatic downgrade is not performed in P9;
- rollback uses the prior signed installer only when explicitly selected/approved;
- schema migration compatibility must be checked before future automatic rollback.

## Acceptance

Automated acceptance:

- frontend/Rust/native build checks green;
- preview packaging workflow syntax and scripts are committed;
- production confirmation gate exists;
- NSIS bundle configuration exists;
- checksum and update metadata generator exists and is tested where practical;
- signing hook fails closed for Production and permits explicitly labelled preview;
- `pc-updater` validates channel policy and checksum.

Manual gates that remain open until evidence exists:

- signed Production artifact generated with real signing material;
- clean Windows installation;
- in-place upgrade preserving local state;
- rollback/downgrade exercise on Windows.

Do not mark those manual gates complete without evidence.
