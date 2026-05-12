## Phase 1: Evidence fixtures

- [x] [serial] Add downstream transport and RPC core fixtures that use only default reusable APIs.
- [x] [depends:fixtures] Capture fixture metadata/check transcripts and forbidden dependency greps.

## Phase 2: Compatibility and checker

- [x] [depends:fixtures] Capture default `cargo tree` evidence for `aspen-transport` and `aspen-rpc-core`.
- [x] [parallel] Capture representative runtime consumer compatibility checks for `aspen-raft-network`, `aspen-client`, and `aspen-rpc-handlers` feature paths.
- [x] [depends:fixtures] Run positive and negative readiness checker evidence for `transport-rpc`.

## Phase 3: Closeout

- [x] [depends:checker] Update manifest, policy, inventory, and verification notes based on the evidence.
- [x] [depends:closeout] Run strict OpenSpec validation, `git diff --check`, and the smallest relevant Nix/Rust checks.
