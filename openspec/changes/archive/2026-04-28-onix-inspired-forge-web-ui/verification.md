# Verification Evidence

## Implementation Evidence

- Changed file: `crates/aspen-forge-web/src/templates.rs`
- Changed file: `README.md`

## Task Coverage

- [x] I1 Add a Forge-local Onix-inspired CSS token layer in `crates/aspen-forge-web/src/templates.rs` covering surfaces, typography, borders, shadows, status colors, code surfaces, and focus states.
  - Evidence: `crates/aspen-forge-web/src/templates.rs`, `openspec/changes/archive/2026-04-28-onix-inspired-forge-web-ui/evidence/implementation-diff.patch`
- [x] I2 Retheme common Forge web components through those tokens: page shell, header/nav, repo cards, tabs, tables, buttons, forms, badges, alerts, code/log blocks, CI stages, and job rows.
  - Evidence: `crates/aspen-forge-web/src/templates.rs`, `openspec/changes/archive/2026-04-28-onix-inspired-forge-web-ui/evidence/implementation-diff.patch`
- [x] I3 Document `../onix-site/` as the Forge web UI reference in `README.md`.
  - Evidence: `README.md`, `openspec/changes/archive/2026-04-28-onix-inspired-forge-web-ui/evidence/implementation-diff.patch`
- [x] V1 Run `cargo nextest run -p aspen-forge-web` and save the transcript under this change's `evidence/` directory.
  - Evidence: `openspec/changes/archive/2026-04-28-onix-inspired-forge-web-ui/evidence/V1-aspen-forge-web-nextest.txt`
- [x] V2 Run `git diff --check` and save the transcript under this change's `evidence/` directory.
  - Evidence: `openspec/changes/archive/2026-04-28-onix-inspired-forge-web-ui/evidence/V2-git-diff-check.txt`
- [x] V3 Run `scripts/openspec-preflight.sh onix-inspired-forge-web-ui` and save the transcript under this change's `evidence/` directory.
  - Evidence: `openspec/changes/archive/2026-04-28-onix-inspired-forge-web-ui/evidence/V3-openspec-preflight.txt`

## Review Scope Snapshot

### `git diff -- README.md crates/aspen-forge-web/src/templates.rs`

- Status: captured
- Artifact: `openspec/changes/archive/2026-04-28-onix-inspired-forge-web-ui/evidence/implementation-diff.patch`

## Verification Commands

### `cargo nextest run -p aspen-forge-web`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-28-onix-inspired-forge-web-ui/evidence/V1-aspen-forge-web-nextest.txt`

### `git diff --check`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-28-onix-inspired-forge-web-ui/evidence/V2-git-diff-check.txt`

### `scripts/openspec-preflight.sh onix-inspired-forge-web-ui`

- Status: pass
- Artifact: `openspec/changes/archive/2026-04-28-onix-inspired-forge-web-ui/evidence/V3-openspec-preflight.txt`

## Notes

- `openspec_gate` reported `unknown active change 'onix-inspired-forge-web-ui'` from this `.pi/worktrees/session-*` checkout, matching the known worktree visibility issue in `AGENTS.md`; local `openspec validate onix-inspired-forge-web-ui --type change --strict --no-interactive` passed before implementation.
