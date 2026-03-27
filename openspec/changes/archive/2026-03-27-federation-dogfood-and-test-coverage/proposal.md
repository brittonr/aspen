## Why

Federation has been built out across 10 changes (handshake through auth tokens) but the pieces haven't been wired together in a two-cluster CI dogfood scenario. The existing dogfood tests (`ci-dogfood-full-loop`) prove single-cluster Forge→CI→Nix build. The existing federation VM tests prove cross-cluster sync and git clone. Neither proves the full loop: Cluster A pushes code, Cluster B federates the repo and runs a CI build from it. That's the gap.

Separately, `aspen-federation` has 216 tests that all pass, but the verified modules (`ref_diff`, `quorum`, `fork_detection`) have no Verus specs for `ref_diff`, and the policy modules have only 20 inline tests across 4 files. The integration tests (wire + auth cluster) are all `#[ignore]` due to iroh socket binding — they run fine locally but aren't covered by any nextest profile automatically.

## What Changes

- **NixOS VM test**: Two-cluster federation dogfood test (`federation-ci-dogfood.nix`). Cluster A creates a Forge repo, pushes a Nix flake, and federates it. Cluster B trusts A, syncs the federated repo, and triggers a CI build from the mirrored content. Verifies the build succeeds and produces a runnable binary.
- **Verus spec for ref_diff**: The `compute_ref_diff` and `resolve_conflicts` functions in `src/verified/ref_diff.rs` lack formal specs. Add `verus/ref_diff_spec.rs`.
- **Unit tests for policy modules**: Add tests for `resource_policy.rs`, `verification.rs`, and `selection.rs` — the modules with the thinnest coverage.
- **Nextest profile**: Add federation `#[ignore]` tests to the `network` profile so they run in `cargo nextest run -P network --run-ignored all`.

## Capabilities

### New Capabilities

- `federation-ci-dogfood`: Two-cluster NixOS VM test proving the Forge→federate→CI build pipeline across independent clusters.
- `federation-ref-diff-verified`: Verus formal specs for `compute_ref_diff` and `resolve_conflicts`.
- `federation-policy-tests`: Unit test coverage for policy modules (resource_policy, verification, selection).

### Modified Capabilities

## Impact

- `nix/tests/`: New `federation-ci-dogfood.nix` VM test
- `crates/aspen-federation/verus/`: New `ref_diff_spec.rs`
- `crates/aspen-federation/src/policy/`: New inline tests
- `.config/nextest.toml`: Updated `network` profile filter
