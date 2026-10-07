# P9 Release and Rollback Policy

## Channels

PC Manager uses three explicit release channels:

- **stable** — production users; accepts only stable releases.
- **beta** — test users; accepts beta and stable releases.
- **dev** — development/preview users; may accept any channel.

A stable client must never automatically consume a beta or dev package.

## Preview artifacts

Preview builds are intentionally unsigned and are named/uploaded as **UNSIGNED PREVIEW**.
They are CI evidence only and must not be distributed as production installers.

## Production release gate

Production is manual-only and must be dispatched from `main` with the exact confirmation:

```powershell
gh workflow run production-release.yml --ref main -f confirm=DEPLOY_PRODUCTION
```

The production job fails closed when the Windows code-signing certificate is unavailable.
It runs the full frontend/Rust release gate before packaging.

Required GitHub Environment secrets:

- `WINDOWS_CODE_SIGNING_PFX_BASE64`
- `WINDOWS_CODE_SIGNING_PFX_PASSWORD`

The PFX is materialized only in the ephemeral runner and removed after bundling.

## Installer and integrity

The production artifact is a signed NSIS Windows installer. Each release includes:

- Authenticode-signed installer;
- `SHA256SUMS.txt`;
- `update-metadata.json`;
- `RELEASE_NOTES.md`.

The metadata schema is `pc-manager.update/v1`. Production channels require signed artifacts.

## State preservation

Installer upgrades must not delete PC Manager local data, operation history, settings, or the
Windows CNG App Manager device key. Release workflows do not run cleanup/reset logic.

This preservation requirement still needs representative upgrade verification on a real Windows
machine before P9 is considered fully accepted.

## Failed update and rollback

P9 does not implement silent automatic rollback. If an update fails:

1. keep the existing installation/state whenever the installer has not committed replacement;
2. show/report the failure rather than deleting local state;
3. allow reinstalling the previous **signed** GitHub release manually;
4. never downgrade automatically based only on remote metadata.

The `pc-updater` policy blocks downgrades by default. A downgrade is an explicit recovery action,
not a normal update decision.

## P10 boundary

P9 produces signed packages and trustworthy metadata. P10 may consume that metadata to implement
software update checks. P10 must verify channel policy, signature requirements and SHA-256 before
handoff to an installer.
