# Design: Make README `cargo run --` commands resolve the molten binary

## Context

Cargo requires `--bin` or `default-run` when a package has more than one binary target. The README documents
`cargo run -- …` throughout.

## Decisions

### Decision: `default-run` instead of editing each README command

**Choice:** Add `default-run = "molten"` to the root `[package]`.

**Rationale:** One manifest line fixes all 44 commands and any future command. `molten-native-extension-fixture` is a
test fixture binary that is never meant to be run by default.

### Decision: No build-plan regeneration

**Choice:** Keep `build-plan.json` and `release-policy-build-plan.json` unchanged.

**Rationale:**
- unit2nix `d4883180` checks staleness in `lib/build-from-unit-graph.nix` lines 97-121, and that check compares only
  `builtins.hashFile "sha256" Cargo.lock` with the plan's `cargoLockHash`.
- The plan's `inputsHash` (which does cover Cargo.toml) is used only by the unit2nix CLI to skip regeneration.
  Neither the flake nor the repository checks it.
- The `git-source-hash-binding` check binds `crate-hashes.json` git sources and does not read Cargo.toml bytes.
- `default-run` adds no target or dependency, so the unit graph is identical.
- `Cargo.lock` is byte-unchanged.

## No-spec classification

Accepted requirement text does not change. Semantic review inputs: the two-file diff, the spot-run outputs, and the
staleness reasoning.

## Failure behavior

`cargo run --bin molten-native-extension-fixture` still selects the fixture explicitly.

## Risks / Trade-offs

- The Octet `config_hash` covers Cargo.toml bytes, so Octet artifacts captured before this commit read as
  `status-config-current: fail`. The strict sequence reruns Octet, so fresh artifacts are current.
