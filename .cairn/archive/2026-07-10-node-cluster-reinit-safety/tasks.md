# Tasks: node-cluster-reinit-safety

## Phase 1: Lifecycle guards

- [x] [serial] r[molten.node_runtime.init_lifecycle_collision_guard] Add a pure node lifecycle-state classifier and deny daemon init when the target root is not empty.
- [x] [parallel] r[molten.node_runtime.cluster_state_root_guard] Reject ambient cluster state roots during cluster planning.

## Phase 2: Cluster reset UX

- [x] [serial] r[molten.node_runtime.cluster_init_reset_guard] Add non-force manifest/lifecycle collision denial and force reset for planned node roots.
- [x] [parallel] r[molten.node_runtime.cluster_init_reset_guard] Document force reset behavior and non-force collision denial in the cluster runbook.

## Phase 3: Positive/negative validation

- [x] [serial] r[molten.node_runtime.init_lifecycle_collision_guard] Add daemon tests for empty, initialized, running, stopped, and inconsistent lifecycle classifications and denial paths.
- [x] [parallel] r[molten.node_runtime.cluster_init_reset_guard] Add CLI tests for manifest collision, lifecycle collision, and force reset of planned node roots.
- [x] [serial] r[molten.node_runtime.cluster_state_root_guard] Ran focused Cargo tests, formatting, Cairn validation/gates, and broader workspace checks. Evidence: `cargo test -q --lib daemon_core::tests` passed 30 tests; `cargo test -q --lib cluster::tests` passed 3 tests; `cargo test -q --test cliharness cli_cluster_init` passed 2 tests; `cargo fmt --check` passed; `nix run path:../cairn#cairn -- validate --root .` and proposal/design/tasks gates passed. Broad `cargo test --workspace` was run and is blocked by existing `pilot::tests::nix_dogfood_release_evidence_verifies_and_denies_stale_refs` (`expected <deterministic-replay-index-v1 ...>`). Broad and lib clippy were run and are blocked by existing unrelated Clippy findings (`cloned_ref_to_slice_refs`/`collapsible_if`) outside this change surface.
