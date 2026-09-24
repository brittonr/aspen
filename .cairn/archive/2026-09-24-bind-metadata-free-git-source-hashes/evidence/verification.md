# Verification: Bind metadata-free git source hashes

Base: `origin/molten` `4e31cee55167a38978961faac5c46476aca6f5ac`. unit2nix `d4883180de0ce3033b7e4e2ab4216f33134863c5`.

## Baseline failure and cause

- `nix flake check --keep-going` on the base: FOD mismatches for `z3kGDT73ja8rewvGLwJFLF5MTx1ih-a0de793` (specified
  `sha256-uyAcv…`, got `sha256-++GX+…`) and `z4ARxEt93T6gJTrTZpUyKHMAemPiv-5abc2b9` (specified `sha256-tr6/U3…`, got
  `sha256-FlNyGe…`), with 18 checks blocked.
- On the base, `nix-store --realise --check` reproduces mismatches for ChaosControl, content-identity, and Choregraph.
  Locally those were masked by existing store paths that contain `.git`.
- unit2nix `src/prefetch.rs:119` uses `--leave-dotGit`. `lib/fetch-source.nix` calls `pkgs.fetchgit` without
  `leaveDotGit`.
- Integrity: the seed and the sibling `durable-authority-state` checkout have the same `a0de793` tree (`e786c62b`),
  and both hash to `sha256-++GX+wiKkcBDeXtMJnMgpByEEcuAYRgO20Pzb8TJ6pA=`.

## Positive

- `scripts/git-source-hashes.sh --write` recorded 16 revision-qualified SRI hashes. The bounded-exec host returned
  HTTP 530, so its hash came from the `flake.lock` `bounded-exec-src` `narHash` locked at the same revision. For
  Artifact, Basalt, and Kamacite, the prefetched values equal their `flake.lock` values.
- Regeneration: `unit2nix --workspace -o build-plan.json` and
  `unit2nix -p molten-release-policy --bin molten-release-policy -o release-policy-build-plan.json`. Relative to the
  base, only git source `sha256`, `inputsHash`, and `workspaceRoot` changed (713 and 254 crates; roots equal).
- `scripts/git-source-hashes.sh --check`: exit 0. `nix build .#checks.x86_64-linux.git-source-hash-binding`: exit 0.
- `nix-store --realise --check` on each FOD, recorded in `evidence/fod-realise-check.txt`: ChaosControl,
  durable-authority-state, bounded-http, content-identity, and Choregraph all realise and recheck with 0 mismatches.

## Negative

- `--check` against the base `build-plan.json` exits 1 and lists every drifted source, for example ChaosControl
  `plan=0csw6d… bound=sha256-rFuxBD…`.
- `--check` against the base `crate-hashes.json` exits 1: sources are missing revision-qualified SRI entries, and
  some keys lack `?rev=`.

## Previously blocked checks (`evidence/previously-blocked-checks.txt`)

16 of 18 build on this branch: `molten-node-host`, `clippy`, all seven `nixos-vm-*`, `release-candidate-binding`,
`release-dependency-profile`, `release-profile-validation`, `requirement-traceability-gate`,
`deterministic-drift-gate`, and `world-operator-profile`, plus the new `git-source-hash-binding`. `nextest`, and
`molten` and `dogfood-local-node` that depend on it, failed in this run on one nondeterministic test:
`fabric_execution::tests::simulation::live_and_simulation_compositions_share_command_and_outcome_shape`, with
"WriteStdin failed: Broken pipe (os error 32)" mapped to `UnknownAfterStart`. That race is independent of this change.
The same derivation passed all 1666 tests in one run on the hash-corrected scratch tree and failed
`live_adapter_bounds_output_and_preserves_publication_failure_receipt` in another. Locally that test fails 1 of 40
runs on the base. It is recorded as a separate follow-up.

## Rerun

Rebuilding `nextest`, `molten`, and `dogfood-local-node` on this branch exited 0. nextest reported "1666 tests run:
1666 passed, 0 skipped". All 18 previously blocked checks and the new `git-source-hash-binding` check therefore build
on this branch. The fabric_execution stdin race still needs its own fix, because it makes a single nextest run fail
nondeterministically.
