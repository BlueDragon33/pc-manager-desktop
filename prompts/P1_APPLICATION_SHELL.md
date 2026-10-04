# P1 EXECUTION PROMPT — Application Shell and Design System

Read `/AGENTS.md`, `/docs/MASTER_BUILD_PROMPT.md`, `/docs/IMPLEMENTATION_PLAN.md`, and `/docs/ARCHITECTURE.md` before changes.

## Goal

Build the durable PC Manager Desktop application shell and design system. P1 is UI architecture only. Do not implement real health, cleanup, startup, monitoring, update, network, or privileged system operations yet.

## Required navigation

Create a native-feeling desktop shell with these destinations:

- Overview
- Health Check
- Smart Clean
- Startup
- Apps
- Storage
- Duplicates
- Monitor
- Restore
- Settings

Use local React state for navigation in P1. Do not add a router dependency unless truly necessary.

## UX requirements

- Original visual identity; do not copy CCleaner artwork, logo, or pixel-identical layout.
- Dark and light themes.
- Compact collapsible sidebar.
- Desktop-first layout from 1366×768 through high-DPI displays.
- Keyboard focus visibility and semantic controls.
- Shared design tokens for spacing, surfaces, borders, typography, status, and radii.
- Shared primitives for cards, section headers, badges, empty states, and placeholder panels.
- Calm language and restrained status colors.
- No fake urgency.
- No fake system values presented as real data.
- P1 placeholders must explicitly say that system data will arrive in later phases.

## Overview page

Provide a polished static shell that previews the future information architecture without pretending values are live.

Suggested sections:

- PC Health summary with "Awaiting first scan"
- category cards: Storage, Performance, Security, Updates, Privacy
- Quick actions to navigate to Health Check, Smart Clean, Startup, and Storage
- Native bridge/app identity status from the existing `get_app_info` command

## Feature pages

Each page should have:

- page title and concise description;
- consistent content width and spacing;
- an honest placeholder/empty state;
- no fabricated scan results.

Settings should include local-only theme selection and sidebar preference in P1.

## State

Persist only harmless UI preferences such as theme and sidebar collapsed state using local storage.

System state must not be mocked as real device state.

## Testing

Add useful unit tests for pure UI state/navigation/theme helpers. Avoid dependency-heavy test infrastructure merely for P1.

Run all existing P0 checks plus frontend tests/build.

## Acceptance gate

P1 is complete only when:

- all required destinations are reachable;
- dark/light themes work;
- sidebar can collapse/expand;
- keyboard focus is visible;
- 1366×768 layout remains usable;
- Overview has honest non-live placeholders;
- existing native bridge status remains available;
- lint/typecheck/tests/build are green;
- Rust checks remain green;
- CI is green;
- P1 checklist is updated only after evidence exists.

Use branch `feat/p1-application-shell` and open a PR into `main`.

Do not proceed to P2 until P1 CI is green and merged.
