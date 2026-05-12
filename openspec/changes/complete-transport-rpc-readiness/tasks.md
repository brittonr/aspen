## Phase 1: Evidence fixtures

- [ ] [serial] Add downstream transport and RPC core fixtures that use only default reusable APIs.
- [ ] [depends:fixtures] Capture fixture metadata/check transcripts and forbidden dependency greps.

## Phase 2: Compatibility and checker

- [ ] [depends:fixtures] Capture default `cargo tree` evidence for `aspen-transport` and `aspen-rpc-core`.
- [ ] [parallel] Capture representative runtime consumer compatibility checks for `aspen-raft-network`, `aspen-client`, and `aspen-rpc-handlers` feature paths.
- [ ] [depends:fixtures] Run positive and negative readiness checker evidence for `transport-rpc`.

## Phase 3: Closeout

- [ ] [depends:checker] Update manifest, policy, inventory, and verification notes based on the evidence.
- [ ] [depends:closeout] Run strict OpenSpec validation, `git diff --check`, and the smallest relevant Nix/Rust checks.
