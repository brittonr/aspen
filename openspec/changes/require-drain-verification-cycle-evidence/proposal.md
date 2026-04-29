## Why

The drain workflow says implementation tasks should run build, test, and format checks, but recent completion evidence only included scoped checks. Drains need either the required full cycle or an explicit scoped alternative/blocker record.

## What Changes

- Add a drain verification matrix that records required `cargo build`, `cargo nextest run`, and `nix fmt` status or justified scoped alternatives.
- Require the matrix before checking final verification tasks.
- Teach review/preflight to flag missing cycle evidence.

## Capabilities

### New Capabilities
- `openspec-governance.drain-verification-cycle`: Drain implementation completion records build/test/format evidence or explicit blockers.

## Impact

- **Files**: drain skill, verification template, preflight/done-review checks.
- **Testing**: Positive full-cycle fixture, positive scoped-alternative fixture with blocker, negative missing-cycle fixture.
