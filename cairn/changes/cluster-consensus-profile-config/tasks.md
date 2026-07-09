# Tasks: cluster-consensus-profile-config

## Phase 1: Config model and manifest adapter

- [x] [serial] r[molten.consensus.cluster_config_selection] Add a typed cluster consensus config value with Raft defaults, profile version override, placement ref, and required evidence refs.
- [x] [serial] r[molten.consensus.cluster_config_selection] Add a manifest builder that derives the consensus algorithm profile from cluster config before constructing the group manifest.

## Phase 2: Runtime config and tests

- [x] [parallel] r[molten.consensus.cluster_config_selection] Thread consensus selection through Nickel-exported runtime startup config validation.
- [x] [serial] r[molten.consensus.cluster_config_selection] Add positive and negative tests for Raft config selection, experimental profile selection, unknown profile denial, runtime config defaulting, and runtime config rejection.
- [x] [serial] r[molten.consensus.cluster_config_selection] Run focused consensus/config tests, formatting, and Cairn validation.

## Implementation notes

- Baseline before implementation: `cargo test engine_registry --lib && cargo test runtime_startup_config --lib` passed; the second filter matched no tests.
- Focused checks after implementation: `cargo test cluster_consensus_config --lib`, `cargo test nickel_export_loads_explicit_consensus_profile_config --lib`, `cargo test nickel_export_rejects_unknown_consensus_profile_config --lib`, `cargo test consensus --lib`, and `cargo test nickel_export --lib` passed.
- Broad check after implementation: `cargo test --lib` passed (900 tests).
- Formatting and lifecycle checks: `cargo fmt --check`, Cairn validate, and Cairn proposal/design/tasks gates for `cluster-consensus-profile-config` passed.
