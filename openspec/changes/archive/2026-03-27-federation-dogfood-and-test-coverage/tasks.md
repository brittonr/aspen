## 1. Verus Spec for ref_diff

- [x] 1.1 Create `crates/aspen-federation/verus/ref_diff_spec.rs` with spec functions modeling ref sets as sequences
- [x] 1.2 Add verified exec function `compute_ref_diff_spec` with ensures clauses proving partition completeness (every ref in exactly one category)
- [x] 1.3 Add verified exec function `resolve_conflicts_spec` with ensures clauses proving conflict resolution preserves partition
- [x] 1.4 Add proof for empty-inputs-produce-empty-output
- [x] 1.5 Register `ref_diff_spec` in `verus/lib.rs` and update invariant documentation
- [x] 1.6 Run `nix run .#verify-verus federation` to confirm all proofs pass

## 2. Policy Module Tests

- [x] 2.1 Add resource_policy tests: empty policy denies, wildcard allows, deny overrides allow, boundary inputs
- [x] 2.2 Add verification tests: valid hash passes, tampered content fails, expired credential rejected, missing signature rejected
- [x] 2.3 Add selection tests: empty seeders returns empty, single seeder selected, untrusted excluded, max count respected
- [x] 2.4 Run `cargo nextest run -p aspen-federation` to confirm all new tests pass

## 3. NixOS VM Test: Federation CI Dogfood

- [x] 3.1 Create `nix/tests/federation-ci-dogfood.nix` with two-VM setup (alice, bob) using pattern from `federation-git-clone.nix`
- [x] 3.2 Wire alice: create forge repo, push Nix flake (aspen-constants crate), federate the repo
- [x] 3.3 Wire bob: trust alice's cluster key, run `federation sync`, verify mirror metadata in KV
- [x] 3.4 Wire bob: trigger CI build on mirrored repo, wait for pipeline completion
- [x] 3.5 Verify CI build output: pipeline success, valid nix store path, runnable binary
- [x] 3.6 Register test in `flake.nix` checks (syntax verified; full build requires --impure --option sandbox false)

## 4. Nextest Profile Cleanup

- [x] 4.1 Verify federation `#[ignore]` tests run under `cargo nextest run -P network --run-ignored all -p aspen-federation`
- [x] 4.2 Document the federation test tiers in `crates/aspen-federation/tests/README.md`: unit (always), integration (network profile), VM (nix build)
