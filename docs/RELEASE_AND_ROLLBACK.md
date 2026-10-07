# P9 Release and Rollback Policy

## Artifact classes

PC Manager has two release classes.

### Unsigned preview

Preview artifacts are engineering evidence only. Their artifact names contain `UNSIGNED-PREVIEW`. They are never promoted automatically and are not a Production release.

### Signed beta/stable

Production-path beta and stable artifacts require the repository signing hook to complete successfully. Missing signing configuration is a hard failure.

## Channel rules

| Channel | Purpose | Automatic install in P9 |
| --- | --- | --- |
| dev | preview and engineering validation | no |
| beta | opt-in release validation | no |
| stable | normal production release | no |

P9 publishes policy and metadata. Automatic install/handoff must remain fail-closed until the signed updater path is fully verified.

## Local state

Installer upgrades must not intentionally remove PC Manager local application state, audit history, operation records, or device identity. State is logically separate from replaceable application binaries.

Uninstall and cleanup of user state are separate explicit concerns; release workflows must never delete state as a recovery shortcut.

## Failed update strategy

1. Stop the update and keep the current installation when pre-install verification fails.
2. Do not delete local state after a failed install.
3. Record/return a bounded failure result.
4. A future updater may retry only from verified metadata and a verified package.

## Rollback and downgrade

P9 does not perform automatic downgrade.

A rollback requires an explicitly selected previously signed installer. Before future automatic rollback is enabled, application/database schema compatibility must be demonstrated for both directions.

## Signing boundary

The repository contains only integration logic. Real certificate/private-key material belongs in protected CI secrets or an external signing service.

Production signing must fail closed if the configured signing material cannot be loaded or if signature verification fails.
