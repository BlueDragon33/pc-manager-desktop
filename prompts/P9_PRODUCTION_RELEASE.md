# P9 EXECUTION PROMPT — Production Packaging and Update Metadata

Read `/AGENTS.md`, `/docs/MASTER_BUILD_PROMPT.md`, `/docs/IMPLEMENTATION_PLAN.md`,
`/docs/ARCHITECTURE.md`, and `/docs/RELEASE_POLICY.md` before changes.

## Goal

Create a reproducible Windows packaging/release pipeline without weakening the P0–P8 trust model.

## Required

- unsigned preview NSIS artifact workflow;
- manual production workflow with exact `DEPLOY_PRODUCTION` confirmation;
- production only from `main`;
- full frontend/Rust checks before packaging;
- Windows Authenticode signing hook that fails closed when credentials are absent;
- SHA-256 checksums;
- release notes;
- `pc-manager.update/v1` updater metadata;
- stable/beta/dev channel policy;
- documented rollback and state-preservation policy;
- production and preview artifacts must be visually/metadata distinguishable.

## Safety

- never publish an unsigned artifact as production;
- never expose signing material in logs or repository files;
- never allow a remote command to supply an installer command line;
- no automatic downgrade;
- no silent state reset during upgrade;
- P10 owns network update discovery/install orchestration.

## Acceptance

Automated acceptance is complete when preview/production workflow definitions, channel policy,
metadata generation, packaging configuration and CI are green. Clean-machine install and real
upgrade state preservation remain explicit manual Windows gates.
