# Verification: exercise simulation faults causally

## Baseline

The pre-change baseline ran on the integration checkout
`/home/brittonr/git/OnixResearch/aspen` at commit `38cb87acd` (branch `molten`), summarized at
`/home/brittonr/git/OnixResearch/aspen/target/octet-baseline-head/summary.txt`: 3619 findings,
0 errors, `warning-only`, tool `cargo-octet 0.1.0`, toolchain
`nightly-2026-03-21-x86_64-unknown-linux-gnu`. The baseline `src/fabric_simulation/tests.rs`
carried one `path_segment_repetition` finding and no `bool_naming` or `excessive_file_length`
finding; the baseline `src/fabric_simulation/composition.rs` carried 10
`unbounded_collection_growth` findings, 2 `explicit_defaults`, 4 `function_length`, 8
`path_segment_repetition`, 2 `excessive_file_length`, and no `non_trait_imports` or `no_unwrap`
finding.

Before any remediation edit, this worktree reproduced the finding list with
`cargo octet check --artifact-dir target/octet-w4-pre`: 3643 findings, 0 errors. A per-`(lint, file)`
diff of `target/octet-w4-pre/summary.txt` against the baseline summary showed exactly 24 added
findings and none removed:

| lint | file | added |
| --- | --- | --- |
| `bool_naming` | `src/fabric_simulation/tests.rs` | 1 |
| `excessive_file_length` | `src/fabric_simulation/tests.rs` | 1 |
| `explicit_defaults` | `src/fabric_simulation/composition.rs` | 4 |
| `no_unwrap` | `src/fabric_simulation/composition.rs` | 6 |
| `non_trait_imports` | `src/fabric_simulation/composition.rs` | 6 |
| `unbounded_collection_growth` | `src/fabric_simulation/composition.rs` | 6 |

## Octet remediation

Every added finding was removed at the source; no lint was disabled, no `#[allow]` was added, no
test was weakened, and `dylint.toml` is unchanged (`[octet] disabled_lints = []`).

- `non_trait_imports` (6): `cargo -q -Zscript scripts/octet-qualify-imports.rs --findings
  target/octet-w4-pre/summary.txt src/fabric_simulation/composition.rs` applied the repository's
  supported repair (14 edits, 0 skipped): the private `use std::collections::BTreeMap` /
  `BTreeSet` / `crate::core_api::world_faults::FaultPhase` imports were deleted and the owner path
  is written at every use site. The two pre-existing `FabricPortError` / `FabricPortResult`
  imports with their documented `tigerstyle::non_trait_imports` reason are untouched.
- `explicit_defaults` (4): the lint rejects `T::default()` for an ADT owned by another crate, so
  `DeterministicSimulationPortRouter::new` now builds `SimulatedTransportState::new()` and
  `SimulatedStorageState::new()`. `crates/molten-core/src/fabric_simulation/adapters.rs` gained
  explicit empty-state constructors for `SimulatedTransportState`, `SimulatedStorageState`, and
  `SimulatedDurableImage`, each listing its fields literally, with `Default` implemented by
  delegating to `new()` so the two pre-existing derived-default call sites in `molten-core` tests
  keep working. The values are identical to the derived defaults.
- `no_unwrap` (6): `reference_world_manifest` and `causal_acknowledgment_manifest` now return
  `crate::error::Result<SimulatedWorldManifest>`, the error type used by every neighbour in the
  module. The reference manifest input propagates with `?`; the transport and durable-state port
  profile lookups return
  `crate::error::MoltenError::invalid_harness("reference world has no <class> port profile")`
  through `ok_or_else`, so a missing profile is a typed failure instead of a panic. Callers were
  updated in the same cutover: `run_reference_shrink_fixture` and `prepare_reference_world`
  propagate with `?`, and the three test call sites in `src/fabric_simulation/tests/causal.rs`
  bind the manifest with an explicit expectation message.
- `unbounded_collection_growth` (8 in the four growth sites the change introduced): the two `Vec`
  sites now reserve from their source bound, `openings` as
  `Vec::with_capacity(self.faults.len())` and `crash_recoveries` as
  `Vec::with_capacity(prepared.world.admitted.manifest.faults.len())`, matching the existing
  `Vec::with_capacity(self.partitions.len())` shape in the adapter. The two `BTreeMap` sites have
  no `with_capacity`, so each carries an explicit local bound check before it grows: `admissions`
  refuses more entries than `manifest.nodes`, and `service_states` refuses more entries than
  `prepared.hosts`. Both are reviewable invariants (at most one admission per node, at most one
  final state per host) and neither changes behavior for an admitted world.
- `bool_naming` (1): the boolean local `retained_kv` is now `has_retained_kv`.
- `excessive_file_length` (1): `src/fabric_simulation/tests.rs` was 307 lines. The four tests added
  by this change moved into `src/fabric_simulation/tests/causal.rs` (144 lines) and the module file
  keeps the pre-existing tests and shared expectations (164 lines) and ends with
  `include!("tests/causal.rs");`. The included file is spliced into the same
  `fabric_simulation::tests` module, so `#[cfg(test)] mod tests;` in
  `src/fabric_simulation/mod.rs` is unchanged, test paths are unchanged, and the module directory
  holds two Rust sources. This matches the existing `src/main/tests.rs` + `src/main/tests/**`
  layout.

## Implementation evidence

- `crates/molten-core/src/fabric_simulation/adapters.rs` (new): `SimulatedTransportState` owns
  pending transmissions with eligibility ticks, drops, deliveries, and partitions with healing;
  `SimulatedStorageState` separates submitted operations, completed operations, and a recoverable
  durable image, and `rebuild_hosts_from_image` reconstructs service state from that image only.
  `simulation_fault_phase` maps every `SimulationFaultKind` onto the existing
  `crate::world_faults::FaultPhase` contract, and `recovery_class_for_operation` reuses
  `expected_recovery_for_phase`, so the simulation does not introduce a second fault taxonomy.
- `src/fabric_simulation/composition.rs`: the router returns typed port failures, holds delayed
  storage completions, drops deliveries, opens and heals partitions, and delivers storage and
  transport completions through the same scheduler boundary as live-shell events. The runner
  selects a recorded choice with `seeded_selection_index(seed, position, eligible.len())` instead
  of the previous constant `None`, advances virtual time through `advance_simulation_time`, and
  records the executed `semantic_output_ref` per choice.
- `crates/molten-core/src/fabric_simulation/replay.rs`: replay comparison reports the first
  mismatching field across position, virtual tick, generation, choice id, semantic output ref, and
  eligible set; minimization reruns each candidate and retains it only when
  `failure_fingerprint` matches the original failing fingerprint (the label-only predicate is
  gone).
- `src/fabric_simulation/canonical.rs` and `reference.rs`: choice records carry the executed
  semantic output ref (a pending or empty ref is rejected), and `ReferenceServiceExecutor::recovered`
  constructs a service from the durable image.

## Checks

Executed in `/home/brittonr/git/OnixResearch/aspen-w4`:

- `cargo fmt --all` then `cargo fmt --check`: clean.
- `cargo check --workspace --all-targets`: finished with no errors.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test -p molten-core --lib`: 372 passed; 0 failed (372 before the remediation edits).
- `cargo test -p molten --lib`: 1462 passed; 0 failed (1462 before the remediation edits).
- `cargo test -p molten-core --lib -- fabric_simulation`: 27 passed; 0 failed.
- `cargo test -p molten --lib -- fabric_simulation`: 10 passed; 0 failed.
- `cargo octet check --artifact-dir target/octet-w4-final`: 3617 findings, 0 errors,
  `warning-only`. By lint:

| lint | final | baseline |
| --- | --- | --- |
| `path_segment_repetition` | 1877 | 1877 |
| `excessive_file_length` | 552 | 552 |
| `borrowed_argument_types` | 368 | 368 |
| `unbounded_collection_growth` | 214 | 216 |
| `function_length` | 204 | 204 |
| `too_many_parameters` | 143 | 143 |
| `no_unwrap` | 80 | 80 |
| `non_trait_imports` | 34 | 34 |
| `usize_in_public_api` | 32 | 32 |
| `explicit_defaults` | 31 | 31 |
| `underscore_in_module_filename` | 21 | 21 |
| `module_file_count` | 18 | 18 |
| `ambient_clock` | 17 | 17 |
| `ambient_env` | 14 | 14 |
| `unbounded_channel` | 9 | 9 |
| `no_recursion` | 2 | 2 |
| `no_panic` | 1 | 1 |

  A per-`(lint, file)` diff of `target/octet-w4-final/summary.txt` against the baseline summary
  reports zero added findings and two removed: `unbounded_collection_growth` in
  `src/fabric_simulation/composition.rs` falls from 10 to 8, because the reference runner's
  observation growth now happens inside a `&mut Vec` helper and is no longer a same-block local
  growth. The change's files carry no `bool_naming`, `no_unwrap`, or `non_trait_imports` finding,
  and `src/fabric_simulation/tests.rs` keeps its single baseline `path_segment_repetition` finding.
- Cairn (`/home/brittonr/git/OnixResearch/cairn/result-cairn/bin/cairn`, policy
  `/home/brittonr/git/OnixResearch/cairn/cairn-policy/generated/cairn-policy.json`):
  `validate --root . --strict` exits 0 with `issues: []`, and `gate proposal|design|tasks
  exercise-simulation-faults-causally --root .` each exit 0 with `issues: []`.

Not run: Nix node-state, dogfood, nextest, and flake checks (see caveats).

## Traceability

`r[molten.fabric_simulation.stateful_transport]`

- `crates/molten-core/src/fabric_simulation/tests.rs`: `transport_delay_postpones_delivery_eligibility`,
  `transport_drop_removes_delivery_and_records_the_drop`,
  `transport_partition_blocks_until_heal_and_delivers_after`,
  `transport_denies_unbounded_pending_and_partition_overflow`.
- `src/fabric_simulation/tests.rs`: `three_reference_services_run_through_host_callbacks_and_named_ports`
  observes delayed-transport port events through the host callback boundary.
- `src/fabric_simulation/tests/causal.rs`:
  `delayed_completion_changes_whether_an_acknowledged_write_survives_a_crash` covers the transport
  and storage faults in one world.

`r[molten.fabric_simulation.stateful_storage]`

- `crates/molten-core/src/fabric_simulation/tests.rs`:
  `storage_completion_holds_head_of_line_and_applies_in_order`,
  `storage_crash_recovers_from_the_durable_image_only`,
  `storage_denies_duplicate_and_unbounded_submissions`.
- `src/fabric_simulation/tests/causal.rs`:
  `delayed_completion_changes_whether_an_acknowledged_write_survives_a_crash` is the acceptance
  test: the acknowledged output ref is identical with and without the delay, the durable run keeps
  the acknowledged key and version 1, the delayed run loses it and reports one
  `AfterPossibleSubmit` lost operation whose recovery class is `Uncertain`, and the two runs have
  different run refs.

`r[molten.fabric_simulation.causal_exploration]`

- `crates/molten-core/src/fabric_simulation/tests.rs`:
  `seeded_selection_exploration_is_deterministic_and_seed_divergent`,
  `scheduler_repeats_canonical_choice_and_replay_rejects_ineligible_choice`,
  `scheduler_denies_stale_generation_and_explicit_bounds`,
  `time_advance_denies_backwards_and_out_of_bounds_ticks`,
  `replay_comparison_identifies_choice_and_length_divergence`,
  `replay_comparison_reports_first_mismatching_time_generation_and_semantic_output`,
  `shrinker_reruns_candidates_and_keeps_only_fingerprint_reproducing_worlds`,
  `shrinker_denies_worlds_whose_failure_does_not_reproduce`,
  `failure_fingerprints_bind_decision_invariants_and_first_failure`,
  `claim_ladder_denies_live_promotion_with_only_simulation_evidence`.
- `src/fabric_simulation/tests/causal.rs`:
  `different_seeds_explore_different_schedules_and_repeat_deterministically`,
  `replay_detects_a_changed_semantic_output_at_the_first_diverging_record`,
  `shrink_reruns_each_candidate_and_keeps_only_reproducing_failures`.
- World-fault contract: `every_fault_kind_maps_onto_the_world_fault_phase_contract`, plus
  `recovery_class_for_operation` assertions in `storage_crash_recovers_from_the_durable_image_only`.
- Bounded non-claims: `differential_is_contract_scoped_and_simulation_cannot_promote_itself_to_live`,
  the `DeterministicWholeSystem` claim profile and `REQUIRED_SIMULATION_NON_CLAIMS` in
  `reference_world_manifest`, and the design's non-claims section.

No invariant failure surfaced by seeded exploration is left untriaged: the reference and causal
runs assert every evaluated invariant passes, and the only failing-invariant case is the
`extension:transactional-key-value:reference-fixture-failing-invariant` fixture that
`run_reference_shrink_fixture` injects deliberately, which
`shrink_reruns_each_candidate_and_keeps_only_reproducing_failures` binds to the retained minimized
case's failure fingerprint.

## Caveats

- Nix checks were not run for this change (no `nix build`, no `nix flake check`, no nextest): no
  dependency, lockfile, or flake input changed, and the parent session reports Nix as not run.
- The Octet total is two below the baseline rather than equal to it. The two removed
  `unbounded_collection_growth` findings are baseline findings whose growth call moved behind a
  helper while the change was written; no finding was traded against them, and no new finding in a
  changed file was accepted.
- Octet numbers come from the `workspace-metadata` profile with `cargo-octet 0.1.0` on toolchain
  `nightly-2026-03-21`; the pre- and post-change summaries were produced by the same tool and
  toolchain as the baseline summary.
- Cairn validation and gates ran through the prebuilt CLI with the sibling checkout's policy
  (`cairn-policy/generated/cairn-policy.json`). The policy pinned by the workspace lockfile
  (`cairn-d7a4d31a0615cac1/3b4c280`) is not parseable by that binary, so those receipts record the
  sibling policy hash rather than the pinned one.
- The two `BTreeMap` bound checks cannot trigger for an admitted world; they exist because the lint
  requires a reviewable bound on map growth, and they fail closed with typed errors if the
  invariant is ever broken.
- `src/fabric_simulation/tests/causal.rs` is included into `fabric_simulation::tests`; test names
  and the `r[verify]` markers are unchanged by the move.

## Claim boundary

The evidence shows that the simulated transport and storage contracts now carry state that changes
delivery and recovery outcomes in the covered paths, that the reference runner selects recorded
seeded choices, that replay stops at the first mismatching field, and that minimization retains
only fingerprint-reproducing candidates. It does not prove live-network or live-storage
equivalence, does not emulate a filesystem, does not benchmark Iroh, Redb, or OS behavior, does not
show that a passing minimized case excludes other failures, and makes no release-eligibility claim.
