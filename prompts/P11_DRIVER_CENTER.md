# P11 — DRIVER CENTER

## Phase intent

Implement a conservative Windows Driver Center after P10 without turning PC Manager into a blind driver installer.

Earlier P9/P10 manual verification gates remain open and must continue to be represented honestly.

## Safety contract

1. Old does not automatically mean bad.
2. Prefer Windows Update / standard Windows surfaces over third-party driver download sites.
3. Never accept arbitrary driver URLs, installer paths, shell strings, PowerShell strings, or package-manager arguments from the frontend or App Manager.
4. Never expose an Update All / mass driver installation action in P11 V1.
5. Never force-install, downgrade, bypass signatures, or install unsigned drivers.
6. Driver age alone must never create an update recommendation.
7. Treat Windows Update driver-class results as availability evidence, not a guarantee that installation is necessary.
8. Use opaque local identifiers for UI selection; do not use raw hardware IDs as authority tokens.
9. Keep scan results bounded and local; do not upload raw driver inventory to App Manager.
10. P11 V1 performs no direct driver mutation. Installation control remains with the standard Windows Optional Updates UI.

## V1 scope

### Installed driver inventory
- Read installed driver metadata from a fixed application-authored Windows provider.
- Show device/display name, manufacturer/provider, version, date where available, class, INF name where appropriate, and signature state.
- Bound the number of returned entries.
- Convert provider failures into structured warnings/errors.

### Available driver updates
- Query the Windows Update Agent for driver-class updates only.
- Do not download or install from the scan.
- Return title, provider/manufacturer/model/class/date and bounded update evidence when Windows exposes it.
- If Windows Update is unavailable, return an explicit unavailable/warning state rather than inventing updates.

### Recommendation semantics
- Never mark a driver bad merely because it is old.
- Explain why an item is shown: Windows Update offers a driver-class update, signature evidence is unavailable, or provider metadata needs user attention.
- Avoid fear-based language.

### User action
- Provide a fixed native action that opens `ms-settings:windowsupdate-optionalupdates`.
- Do not pass user-controlled URIs or command lines.
- Do not install a driver directly in P11 V1.
- Because no direct mutation occurs, do not fabricate a restore point or rollback record.

### UI
- Add Driver Center navigation.
- Show provider state, installed-driver summary, available update count, warnings, and filtering.
- Clearly distinguish installed inventory from Windows Update availability evidence.
- State that installation remains controlled by Windows.
- No Update All button.

## Acceptance criteria

- frontend lint/typecheck/tests/build/format are green;
- Rust fmt/check/tests/Clippy are green;
- Windows smoke test exercises the read-only provider without installing anything;
- provider outputs are bounded;
- unit tests cover opaque IDs, unavailable provider behavior, old-driver non-recommendation, and result mapping;
- no arbitrary remote command, URL download, shell primitive, direct driver install, downgrade, or unsigned-driver bypass exists;
- manual real-Windows gate remains open until representative inventory and Optional Updates handoff are verified.
