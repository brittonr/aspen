# F02 startup evidence prerequisite review — 2026-10-21

## Outcome

Blocked, not implemented or accepted. Normal startup still supplies synthetic source-gate evidence. Replacing it with the current receipt validator would not establish the exact-candidate binding required by this change.

Reviewed product source: `f8ccefe6031b6746d2c8661f4599d1cdcf9bfa74`.

## Executed observations

From the isolated change worktree, inside the repository Nix shell:

- `cargo test -p molten --lib node`: 64 passed.
- `cargo test -p molten --lib source_gate`: 9 passed.
- `cargo test --test cliharness cli_node_init_run_status_and_stop_write_receipts -- --exact --nocapture`: 1 passed. This existing integration test runs the real normal `node init`/`node run` CLI in a fresh owned workspace without supplying independently produced gate artifacts. Startup succeeds. Its source helper is `tests/parts/cliharness/p011/body.rs::start_case`.

The CLI observation exercises the artifactless acceptance path. It does not validate a positive real-artifact control or prove source-to-binary linkage. Existing gate unit fixtures are not real Octet execution evidence.

The first Nix-shell launch failed because `/tmp` exhausted its quota. Subsequent commands used a task-owned `TMPDIR` outside that volume; no shared files were deleted.

## Verified prerequisite gaps

- `src/node/parts/daemon/p018/body.rs::run_local_with_root` supplies `synthetic_clean_octet_gate_receipt_for_tests`. It also calls `verify_restart_state` before gate admission. No startup fix is claimed.
- `src/octet/parts/gate/p007/body.rs::capture_workspace` captures only `Cargo.toml` and `dylint.toml`. Its configuration/profile hashes do not identify Rust source or the executing binary.
- `src/octet/parts/gate/p006/body.rs::validate_object_corpus` validates schema, nonempty count, fingerprint syntax and declared source-path coverage. It does not recompute the corpus fingerprint from current source.
- `src/octet/parts/gate/p004/body.rs::validate_octet_source_gate` captures workspace I/O and validates receipt metadata/checks. It is not the proposed pure candidate-admission boundary.
- `src/cli/workflow/provenance/ops.rs::build_record` records caller-supplied source/artifact refs. `verify_build` likewise receives the actual-artifact ref from its caller; these CLI operations do not build or identify the running executable.
- `flake.nix::releaseCandidateBindingCheck` explicitly exercises a conformance fixture using declared candidate/artifact refs. That check is not a producer of current executable provenance.

## Required handoff

Node-runtime and Octet/build integration maintainers need a reviewed producer contract that binds the actual compiled candidate, exact source/dependency closure, policy, profile, toolchain and complete gate-artifact cohort. Startup and compatibility wrappers must select and independently validate that same binding. A normal-path positive control must use this producer before F02 can be completed.

Package version, a toolchain-label blacklist, configuration freshness, matching caller-supplied refs, or a newly invented passing receipt cannot replace this handoff. Adding an authoritative build-time binding is a mechanism/design decision, not a reason to mark the current evidence sufficient.

F02 remains active with `status: blocked`. No requirements are synced, no archive or completion is claimed, and no production fallback or denial-only partial implementation has been introduced.
