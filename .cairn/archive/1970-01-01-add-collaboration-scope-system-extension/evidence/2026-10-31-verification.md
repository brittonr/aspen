# Verification: collaboration-scope system extension closeout (2026-10-31)

Worktree `/home/brittonr/.cargo-target/aspen-collab-20261031`, branch
`cairn/collaboration-scope-closeout-20261031`, base `c267d062c`. Every Cargo command ran as
`env XDG_CACHE_HOME=… TMPDIR=… CARGO_TARGET_DIR=/home/brittonr/.cargo-target/aspen-collab-target-20261031
CARGO_BUILD_JOBS=8 CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 nix develop path:. --builders '' -c <cmd>`.
Only commands listed here were run; results are copied from their output.

## Baseline on the base commit

`cargo test --workspace --no-fail-fast` on `c267d062c` (integration worktree, same tree):
2911 passed, 0 failed, 28 ignored. Every collaboration test in that run passed, including
`fabric_consistency::raft::collaboration_process::three_participant_conflict_partition_restart_snapshot_and_currentness_recovery`.

## Source changes in this closeout (tests only)

| File | Adds |
|---|---|
| `src/collaborationscope/admission/manifesttests.rs` | manifest admission positive and negatives |
| `src/collaborationscope/admission/installationtests.rs` | `Installation::from_host` against a real admitted host and opened group |
| `src/collaborationscope/codec/projectiontests.rs` | operator status, consumer views, snapshot leak rejection |
| `src/collaborationscope/properties.rs` | two Hegel properties (64 cases each) |
| `crates/molten-core/src/collaborationscope/tests.rs` | wrong-scope and wrong-resource audience denials; a fabric node identity without membership is denied |
| `crates/molten-core/src/collaborationscope/tests/transitions.rs` | an earlier-epoch audience request is stale after removal |

Production source is unchanged.

## Focused tests on the final source

| Command | Result | Log |
|---|---|---|
| `cargo test -p molten-core --lib collaboration_scope` | ok, 11 passed, 0 failed | `2026-10-31-molten-core-collaboration.log` |
| `cargo test -p molten --lib collaboration_scope::` | ok, 26 passed, 0 failed | `2026-10-31-molten-lib-collaboration.log` |
| `cargo test -p molten --lib fabric_consistency::raft::collaboration_process::three_participant_conflict_partition_restart_snapshot_and_currentness_recovery -- --exact`, run 1 | ok, 1 passed | `2026-10-31-three-process-run-1.log` |
| same, run 2 | ok, 1 passed | `2026-10-31-three-process-run-2.log` |

The 26 shell tests include the deterministic installed-profile simulation
(`same_application_world_replays_partition_crash_conflict_and_convergence`,
`actual_simulated_response_loss_quarantines_reloads_and_snapshot_admission`, installed-world
admission mutants) and the native runtime configuration tests.

### Property mutation check

To show the two properties are not vacuous, `transition.rs` was temporarily changed so that
`advance` adds two epochs once `revision > 4` and `bind` no longer rejects an already bound
resource. `cargo test -p molten --lib collaboration_scope::properties` then reported
`FAILED. 0 passed; 2 failed`. The file was restored (`git diff --quiet` on it succeeded) and
both properties pass on the real core (above).

## Static checks

| Command | Result |
|---|---|
| `cargo fmt --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo octet check --artifact-dir …/octet` | `Status: clean`, `Findings: 0` |
| `cargo octet check -p molten --artifact-dir …/octet-lib -- --lib` | `Status: clean`, `Findings: 0` |

These ran again after the scenario markers and the last two core assertions were added: fmt, Clippy and the primary Octet run stayed clean, the core suite reported 11 passed and the shell suite 26 passed.

The first Octet run on the new tests reported ten findings (compound assertions, two helper
names repeating the module segment, two unbounded pushes); they were fixed in the tests, and a
first Clippy run rejected `.err().expect()`; both were rerun clean as above.

## Identities

| Item | Identity |
|---|---|
| Extension | `molten.collaboration-scope`, state schema `molten.collaboration-scope.v1` |
| Commit port | `molten.collaboration-scope.commit`; input `molten.collaboration-scope.staged-command.v1`, output `molten.collaboration-scope.committed-command.v1` |
| Basalt and UCAN | `basalt`, `ucan`, `ucan-core` 0.1.0 at `89675cd4f585f837323c049e4a25f7b94c903038` (`Cargo.lock`) |
| Fixture policy | `fixtures::POLICY` (`ucan-nickel-contracts.policy.v1`, contract `collaboration-scope`); test-only |
| Simulation profile | `collaboration-installed-simulation-profile-v1`, derived at run time from the BLAKE3 source refs in `simulation/world.rs` (`core_identity`, `extension_profile`) |
| Consumer views | `lattice-effect-attempt`, `animus-context-refs`, `tile-read-only-status`, marker `revalidate-before-effect-not-bearer-authority` |
| Snapshot marker | `one-request-evidence-not-bearer-authority` |

BLAKE3 of the governing sources on the final tree:

```
579594f609249b38e7826ca7a1786f1875c8251de9c63439b18e91998e424681 crates/molten-core/src/collaborationscope/identity.rs
f6572901350fa5b3653d96f055f9fa734e92e4ab8053ae0829ecb591928a9664 crates/molten-core/src/collaborationscope/model.rs
1e52fd1e9b6a489d69e7f5b0411d45d9c5398d94b01e7bad466a0353052fd344 crates/molten-core/src/collaborationscope/admission.rs
0005e87f6c98b9f1c1e7a082c87f5b049342fca76b007e1866439adfc6ce5aa1 crates/molten-core/src/collaborationscope/transition.rs
fa695d22e5d155b6a6d54b55ac18234ca5d8205afd7bb8921febc262ac6a5913 src/collaborationscope/codec/canonical.rs
f8779e0606e64d751beb444d796d0e48fdf20be61692b34fa224836c677e9686 src/collaborationscope/codec/command.rs
c429dffed09c81544b4fc27d2874a7aa47a77aff2407cf4759628b857cfe4e60 src/collaborationscope/codec/decode.rs
436ffaa34df731bf671a0e6142ce34744b5accd15447be390a4f4c273335b504 src/collaborationscope/codec/projection.rs
fcf4a770974b191fd798c9830888e3d21d76bfd0e1873723123105287fa2ad5b src/collaborationscope/admission/manifest.rs
d8e18cc1453517fd345730da8158f3f5e9c6a66af0db15ee422629b491e276fd src/collaborationscope/admission/binding.rs
42169d3e15993450bc2dc59f7d1e3d3681c5573b9b17fbbd963e620ac3a15536 src/collaborationscope/admission/authority.rs
e81efc07945b399a7bc37db2052e8b010f88f3d949f33a5b1df1ed747da92f48 src/collaborationscope/simulation/world.rs
aec0fd1e46005f83f4026a2841ce8707b0b147f802a373e06425119b31976cdf src/collaborationscope/fixtures.rs
```

## Not executed here

- Installed native executable acceptance (`molten-collaboration-scope` through the
  source-gated retained public host with independently signed operator and scope credentials):
  owned by `route-installed-native-effects`, still open.
- No NixOS VM, multi-host network, production policy or production key was used. The
  three-process scenario runs on one host; the simulation replays recorded delivery.

## Lifecycle

Through `nix run path:/home/brittonr/git/OnixResearch/cairn#cairn -- <command> --root .
--policy /home/brittonr/git/OnixResearch/cairn/cairn-policy/generated/cairn-policy.json`; JSON
outputs are retained under `2026-10-31-closeout/`.

| Command | Result | Retained |
|---|---|---|
| `validate --strict` (before closeout) | `valid: true`, 0 issues | — |
| `gate proposal\|design\|tasks add-collaboration-scope-system-extension` | each exit 0, `issues: []` | `gate-*.json` |
| `sync …` (plan) | `blocked: false`, no merge diagnostics, 9 added requirements | `sync-plan.json` |
| `sync … --execute` | `mutated: true`, new `.cairn/specs/collaboration-scope/spec.md` | `sync-execution.json` |
| `archive …` (plan, then `--execute`) | `blocked: false`; moved to `.cairn/archive/1970-01-01-add-collaboration-scope-system-extension` | `archive-plan.json`, `archive-execution.json` |
| `validate --strict` (after archive) | `valid: true`, 0 issues | `postarchive-validation.json` |
| `nix build path:.#checks.x86_64-linux.inherited-tracey-debt` | first run: `error: traceability debt differs from the reviewed baseline` (19 accepted scenario ids had no reference); after adding `r[verify …]` markers on the tests that check each scenario: `verdict=pass`, `dangling=0` | — |

