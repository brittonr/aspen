## Why

Aspen uses Verus for function-level formal proofs and madsim for deterministic simulation, but has no protocol-level model checking. Verus proves individual functions correct; madsim runs specific scenarios. Neither exhaustively explores all possible message orderings, crash sequences, and state transitions the way TLA+ does. Oxide's trust-quorum TLA+ spec caught real bugs in their reconfiguration protocol — message ordering issues and crash recovery races that would be invisible to conventional testing.

The trust protocol being added to Aspen (`shamir-cluster-secret`, `secret-rotation-on-membership-change`, `node-expungement-protocol`) is exactly the kind of distributed protocol where TLA+ model checking provides the most value: multiple participants, crash recovery, and security-critical state.

## What Changes

- Add TLA+ specs for Aspen's trust protocol (init, reconfiguration, share collection, expungement)
- Add TLA+ spec for Raft membership changes interacting with trust epochs
- Set up TLC model checker in CI via Nix (deterministic, reproducible)
- Define safety invariants: share consistency, epoch monotonicity, expungement permanence
- Define liveness properties: reconfiguration eventually completes when quorum is available
- Document TLA+ conventions and how specs relate to Rust implementation

## Capabilities

### New Capabilities

- `tla-model-checking`: TLA+ specifications for Aspen's trust protocol with TLC model checking, safety invariants, and liveness properties, integrated into CI via Nix

### Modified Capabilities

## Impact

- New directory: `tla/` at repo root (or `crates/aspen-trust/tla/`)
- Nix flake: add TLC/TLA+ tooling, `nix run .#check-tla` app
- CI: model checking as a `nix flake check` output
- Documentation: `docs/tla-specs.md` with conventions and spec-to-code mapping
