# Tasks

- [x] [serial] Replace unframed `\0`/`|` hashing in `LiveWorkload::workload_ref` and `failure_fingerprint` (`crates/molten-core/src/fabric_simulation/{live_differential.rs,replay.rs}`) with collision-free canonical framing and domain separation, reject NUL bytes in `valid_ordering_key`, validate `simulated_class` in `evaluate_live_differential`, and compare full choice metadata in `compare_replay`. r[molten.fabric_simulation.collision_free_workload_and_complete_comparison]
- [x] [serial] Traverse Preserves `Embedded` values in `src/preserves/parts/rail/p008/body.rs`, enforce unique `<some>`/`<none>` optional refs (`rail/{p003,p006}`, `secrets/p006`), and enforce lowercase-only `blake3:` hex validation across `crates/molten-core` and `src/addressable_actor/ports.rs`. r[molten.runtime_spine.canonical_preserves_identity_and_embedded_scan]
- [x] [parallel] Replace `serde_json::to_vec` identity hashing (`src/world_snapshot/{chaoscontrol.rs,vm/helpers.rs}`) and `format!("{issue:?}")` receipt hashing (`src/worldcommit/{records.rs,shell.rs}`, `crates/molten-core/src/worldcommit/restore.rs`, `src/world_head/parts/service/p001/body.rs`) with canonical packed Preserves and closed typed issue tags. r[molten.runtime_spine.canonical_preserves_identity_and_embedded_scan]
- [x] [serial] Fix Runtime Dataspace (`src/runtime/dataspace/parts/state/{p000..p004}/body.rs`): checked monotonic `logical_time`/`effect_sequence`, recorded `Effect::Random` `value < upper` check, honest rollback receipt without `TurnCommitted`, real actor IDs and observer retraction notifications on facet turns/stops, and `ForceRun` precedence over `DependencyGated`. r[molten.runtime_spine.dataspace_determinism_and_rollback_integrity]
- [x] [parallel] Fix World Replay (`src/world_replay/boundary/sealed.rs` validate-before-pop + violation tracking, `adapters.rs` trace-to-record binding, `crates/molten-core/src/world_replay/comparison.rs` full root/field comparison). r[molten.world_commit.replay_sealed_observation_and_full_comparison]
- [x] [parallel] Fix Prolly Map `rebalance_last_group` (`crates/molten-core/src/prolly_map/tree/build.rs`) to enforce `max_node_bytes`, and enable `cranelift_nan_canonicalization(true)` plus synchronous single-call enforcement in `src/harness/wasm/parts/executor/p000/body.rs`. r[molten.prolly.rebalance_enforces_max_node_bytes] r[molten.wasm.harness_nan_canonicalization_and_sync_call_limit]
- [ ] [serial] Add positive/negative regression and Hegel property tests for every defect above and run focused suites, Clippy, Octet, and strict Cairn validation. r[molten.runtime_spine.canonical_preserves_identity_and_embedded_scan] r[molten.runtime_spine.dataspace_determinism_and_rollback_integrity] r[molten.fabric_simulation.collision_free_workload_and_complete_comparison] r[molten.world_commit.replay_sealed_observation_and_full_comparison] r[molten.prolly.rebalance_enforces_max_node_bytes] r[molten.wasm.harness_nan_canonicalization_and_sync_call_limit]

## Verification (2026-11-26, tasks 4–6: dataspace, world replay, prolly rebalance, Wasm harness)

Implementation notes:

- Dataspace: live and recorded clock/sequence advances go through `next_restorable_counter` (checked, never reaching the `u64::MAX` that restore rejects; no partial state change on exhaustion). Recorded clock values below `logical_time` and recorded random values outside `0..upper` are denied; a live random step with `upper == 0` is denied too, so every live answer stays replayable. `begin_effect_for_step` now returns `Result<Option<Event>>` (callers in the harness replay runner and both world-snapshot fixtures updated). `commit_turn_with_predicate_receipt` routes a staged rollback (retraction of an unheld assertion) to a rollback evaluated for `TurnOutcome::RolledBack`: only `TurnRolledBack` is returned, never `TurnCommitted` or a committed-outcome receipt. Facet turns are attributed to the facet owner; facet stop emits `AssertionRetractionObserved` to surviving observers when the last visible owner goes. `ServiceDemandSet::derive` ranks demand forms explicitly.
- The `hegel_mixed_turn_commit_and_rollback_trace_preserves_transition_laws` property encoded the old behavior (it committed generated retractions of never-asserted values with a committed receipt); it now expects the rolled-back outcome for staged rollbacks.
- World replay: `SealedReplayObservations::observe` validates before removing and counts denied demands; `finish_step` fails on any. `world_replay_trace_member` re-encodes the validated trace and requires equal record bytes and record ref. `compare_step` reports a root divergence for differing roots or any field difference under an equal commit ref.
- Prolly: `rebalance_last_group` borrows or merges only when the result fits both `max_fanout` and `max_node_bytes`; otherwise the last group stays underfull (readers require no minimum fanout). The prior hard `InternalFanoutExceeded` on an over-fanout merge is replaced by the same rule. molten-core has no `hegeltest` dependency, so the property is exhaustive deterministic cases.
- Wasm harness: `cranelift_nan_canonicalization(true)`; the hostcall trampoline admits exactly one call of the step's operation and denies any other call synchronously before recording it or writing a response.

Commands (run on leviathan through `aremote`, formatting locally):

- `cargo fmt --all` (local): clean.
- `cargo test -p molten-core --lib -- world_replay prolly_map`: 57 passed, 0 failed.
- `cargo test -p molten --lib -- engine::dataspace engine::logic testbed:: world_replay world_snapshot meshspace::recorded`: 253 passed, 0 failed (first run surfaced the stale mixed-turn property above, fixed and rerun).
- `cargo clippy -p molten -p molten-core --all-targets -- -D warnings`: exit 0, no diagnostics.
- Mutation check: with `cranelift_nan_canonicalization(false)`, `testbed::tests::wasm_nan_results_are_canonical_and_replay_identically` fails (guest observed the non-canonical x86 NaN); restored to `true`.
- After restoring: `cargo test -p molten --lib -- testbed::tests::wasm_`: 14 passed; `cargo test -p molten --test cliharness`: 51 passed, 13 ignored; `cargo test -p molten --test worldsnapshot`: 13 passed.
- `cairn validate --root . --policy …/cairn-policy.json --strict` (local): valid.

Task 7 (repository-wide Octet/Hegel/strict-validation sweep across all six requirements) is left unchecked for the integrating parent.

## Verification (2026-11-26, tasks 1–3)

Implementation notes:

- Task 1: `workload_ref` and `failure_fingerprint` hash length-framed fields under BLAKE3 `derive_key` domains (`live-workload.v2`, `failure-fingerprint.v2`) via `fabric_simulation/framing.rs`; failed invariants are sorted and deduplicated; `valid_ordering_key` rejects every `char::is_control` character; a simulated `Unsupported` class is `Incomplete` and a simulated class outside `declared.permitted` is `Failure`; `compare_replay` compares selected `kind`/`node_id`/`ready_at_tick` and full eligible metadata (`eligible-metadata`). Molten-core has no `hegeltest` dependency, so its regressions are deterministic tests.
- Task 2: `container_children` descends into `Embedded` payloads (path segment `embedded`, bounded by depth/node limits). The bare-string `Some` fallback is removed from `optional_content_ref`, `validate_optional_ref_boundary_record`, and secrets `parse_optional_ref`; all in-tree producers already emit `<some ...>`/`<none>`. Lowercase-only hex is enforced in `codec.rs`, `stack.rs`, `fabric_time/mod.rs`, `fabric_time/reservation/batch.rs`, `fabric_simulation/admission.rs`, `world_witness/reconciliation.rs`, `addressable_actor/ports.rs`, and additionally `world_head/choregraph.rs` (its `u8::from_str_radix` also admitted uppercase and a `+` sign). `crates/molten-core/src/preserves_profile.rs` was left untouched because the concurrent modularity change deletes that file (no production callers; its Nickel contract already requires `^blake3:[0-9a-f]{64}$`).
- Task 3: ChaosControl fact/component identities (domains bumped to `v2`) and VM Cohort plan identities (`vm-cohort-plan.v2`) hash the canonical packed bytes of explicit Preserves projections built in `world_snapshot/chaoscontrol.rs` and `world_snapshot/vm/helpers.rs` (`cohort_plan_value`, `guest_artifact_value`, …). The VM `mechanism_plan_json` handoff payload remains JSON and is not identity material. `CaptureIssue::code`, `ClosureIssue::code`, and `WorldHeadIssue::code` give closed kebab-case tags; `ClosureIssue::InvalidCore` now carries a typed `CaptureIssue` instead of a `Debug` string. `Cargo.toml` and `Cargo.lock` are unchanged. No checked-in fixture pinned the changed identities; `tests/fixtures/fabric-boundary/` is unchanged.

Commands (all on leviathan through `aremote`, worktree `aspen-swarm-canonical-identity`):

- `cargo test -p molten-core --lib fabric_simulation` — 46 passed, 0 failed.
- `cargo test -p molten-core --lib` — 807 passed, 0 failed.
- `cargo test -p molten --lib -- embedded optional_ref optional_content serialized_identity actor_references closure_report_receipt chaoscontrol_fact secret_optional` — 18 passed, 0 failed (includes the four new Hegel properties).
- `cargo test -p molten --lib` — 2220 passed, 2 failed, 11 ignored; the two failures (`fabric_execution::tests::live::adapter_clears_environment_round_trips_input_and_publishes_bounded_output` stdout-teardown timeout and `transforms::tests::hegel_preview_apply_consistency_and_path_stability` Hegel `TooSlow` health check) are load-timing failures in untouched modules; see the rerun below.
- `cargo test -p molten --lib --features world-snapshot-vm-cohort -- world_snapshot` — 23 passed, 0 failed.
- `cargo clippy -p molten -p molten-core --all-targets -- -D warnings` — exit 0.
- `cargo fmt --all` (local) and `cairn validate --root . --policy …/cairn-policy.json --strict` (local) — valid.
- Rerun `cargo test -p molten --lib -- fabric_execution::tests::live::adapter_clears_environment transforms::tests::hegel_preview_apply` — 2 passed, 0 failed.
- `cargo test -p molten --test {worldcommit,worldsnapshot,fabricboundarycompat,fabric_simulation_boundary,worldpromotiondenials,worldpromotioncli}` — 4, 13, 5, 3, 2, 5 passed; 0 failed.
- `cargo test -p molten --test cliharness -- fabric_simulation repro_export secret world` — 3 passed, 0 failed.
- Octet was not run for this slice; the final task 7 rail remains with the change owner.

Review fixes (2026-11-26, FIX-FIRST round):

- Superseded in the second review round: the generic serde helper (`canonical_serialized_bytes` with an `OrderedSerde` bound and manual impls on foreign DTOs) was removed, together with its two Hegel properties and the explicit `preserves` `serde` feature. Foreign DTOs are now projected field by field into Preserves records; every struct is destructured without `..` and every enum is matched without a wildcard, so an upstream field, variant, or collection-type change fails to compile instead of silently changing an identity, and every collection is an ordered sequence. Tests: `chaoscontrol_fact_and_component_identities_hash_the_explicit_preserves_projection` and `vm_cohort_plan_ref_hashes_the_explicit_preserves_projection` (`world-snapshot-vm-cohort`) rebuild the expected projection independently from the fixture and match the emitted refs.
- Remaining uppercase-accepting `blake3:` digest boundaries fixed: `spacewasm::evidence::validate_pins`, `remote` dataspace `references_in_text`, `world_witness` provider `claim_digest`, and recovery `identity`. Generic payload hex decoders (Wasm module bytes, signatures, public keys, git revisions) are not `blake3:` identities and are unchanged. Tests: `authority_pins_reject_uppercase_digest_hex`, `reference_scanning_ignores_uppercase_digest_spellings`.
- `WorldHeadIssue::InvalidBounds` now carries the closed `WorldHeadBound` enum with kebab-case codes (`max-signers`, `policy-max-conflicts`, `conflict-identity-length`, …); every producer (`admission.rs`, `choregraph.rs`, `conflict.rs`) uses it, and the previously duplicated bounds/policy `max_conflicts` tags are now distinct. Test: `invalid_bounds_denial_receipt_carries_closed_kebab_case_codes` executes a real `execute_world_head_transition` denial and checks the receipt text.
- `cargo test -p molten-core --lib` — 807 passed, 0 failed.
- `cargo test -p molten --lib -- codec:: vault:: world_head spacewasm remote world_witness world_snapshot world_commit addressable_actor` — 182 passed, 0 failed.
- `cargo test -p molten --lib -- invalid_bounds_denial_receipt authority_pins_reject_uppercase reference_scanning_ignores_uppercase hegel_equal_sets_inserted hegel_serialized_identity` — 4 passed, 0 failed (the spacewasm test is feature-gated; see below).
- `cargo test -p molten --lib --features world-snapshot-vm-cohort -- world_snapshot` — 23 passed, 0 failed.
- `cargo clippy -p molten -p molten-core --all-targets -- -D warnings` and `cargo clippy -p molten --lib --features world-snapshot-vm-cohort -- -D warnings` — exit 0.
- `cargo test -p molten --lib --features spacewasm-pilot -- spacewasm::evidence` — 5 passed, 0 failed; `cargo clippy -p molten --lib --tests --features spacewasm-pilot -- -D warnings` — exit 0.

Second review round (explicit DTO projections):

- `cargo test -p molten --lib --features world-snapshot-vm-cohort -- world_snapshot codec::` — 58 passed, 0 failed.
- `cargo test -p molten --lib -- world_snapshot` — 19 passed, 0 failed.
- `cargo test -p molten --lib --features world-snapshot-vm-cohort -- hash_the_explicit_preserves_projection plan_ref_hashes_the_explicit` — 2 passed, 0 failed.
- `cargo clippy -p molten -p molten-core --all-targets -- -D warnings` and `cargo clippy -p molten --lib --tests --features world-snapshot-vm-cohort -- -D warnings` — exit 0.

Octet round: the ChaosControl projections moved to `src/world_snapshot/chaoscontrol/projection.rs` (no behavior change; `chaoscontrol.rs` 284 lines). Test-only Octet findings fixed: the actor digest tests folded into `addressable_actor/tests/records.rs` (`references.rs` removed), the Embedded scan tests moved to rail test shard `tests/m000/p003`, the world-head bounds-denial test moved to `world_head/parts/tests/p002`, and compound assertions split.

- `cargo octet check --artifact-dir target/octet` — status clean, 0 findings.
- `cargo test -p molten --lib --features world-snapshot-vm-cohort -- hash_the_explicit_preserves_projection plan_ref_hashes_the_explicit invalid_bounds_denial embedded uppercase_digest closure_report_receipt` — 16 passed, 0 failed.
- `cargo clippy -p molten -p molten-core --all-targets -- -D warnings` and `cargo clippy -p molten --lib --tests --features world-snapshot-vm-cohort -- -D warnings` — exit 0.

## Verification (2026-11-26, final validation)

Coverage was checked against the code: each defect has positive and negative regressions that fail on the old behavior. Hegel properties were missing for the recorded-response, demand-precedence and sealed-observation defects in the root crate. They are added in new files (below), with no change to existing tests. `molten-core` has no `hegeltest` dependency, so its defects (framing, comparison, Prolly) keep deterministic and exhaustive-case tests.

1. Workload and fingerprint framing: `live_workload_ref_frames_fields_so_a_forged_ordering_key_cannot_collide` (rebuilds the legacy unframed material to show the old collision), `live_workload_rejects_nul_and_control_bytes_in_ordering_keys`, `live_workload_ref_distinguishes_absent_and_present_ordering_keys_and_permitted_lists`, `failure_fingerprints_frame_invariants_so_joined_ids_cannot_collide`, `live_differential_rejects_unsupported_or_unpermitted_simulated_classes`, `replay_comparison_detects_selected_and_eligible_choice_metadata_divergence`.
2. Embedded scans: `structural_scan_descends_into_embedded_values`, `structural_scan_counts_embedded_layers_against_the_depth_bound`, `hegel_embedded_markers_and_refs_never_evade_structural_scans`.
3. Identity encoding: `chaoscontrol_fact_and_component_identities_hash_the_explicit_preserves_projection`, `vm_cohort_plan_ref_hashes_the_explicit_preserves_projection` (feature `world-snapshot-vm-cohort`), `closure_report_receipt_identity_uses_typed_issue_codes_not_debug_output`, `closure_and_capture_issue_codes_are_closed_typed_tags`, `world_head_issue_codes_are_closed_kebab_tags`, `invalid_bounds_denial_receipt_carries_closed_kebab_case_codes`.
4. Optional refs and lowercase hex:
   - Optional refs: `optional_content_refs_have_one_canonical_encoding`, `hegel_optional_content_refs_reject_bare_and_uppercase_digests`, `secret_optional_refs_reject_bare_strings_as_some`.
   - Lowercase hex: `domain_codec_facade_rejects_uppercase_digest_hex`, `simulation_refs_reject_uppercase_digest_hex`, `stack_evidence_refs_reject_uppercase_digest_hex`, `uppercase_enforcement_digest_is_not_an_admitted_termination_fact`, `uppercase_operation_digest_does_not_bind_the_reconciliation_identity`, `choregraph_digests_accept_only_lowercase_hex_pairs`, `uppercase_digest_hex_is_rejected`, `hegel_uppercase_digest_letters_are_rejected`, `reference_scanning_ignores_uppercase_digest_spellings`, `authority_pins_reject_uppercase_digest_hex` (feature `spacewasm-pilot`).
5. Dataspace: `recorded_clock_regression_and_exhaustion_are_denied`, `recorded_random_value_must_lie_below_its_upper_bound`, `live_counters_never_reach_an_unrestorable_maximum`, `rolled_back_retraction_returns_only_a_rollback_receipt` (plus the updated `hegel_mixed_turn_commit_and_rollback_trace_preserves_transition_laws`), `facet_turns_are_attributed_to_the_owning_actor`, `facet_stop_notifies_observers_when_the_last_owner_retracts`, `force_run_demand_outranks_dependency_gated_in_any_order`.
   - New Hegel properties (`src/runtime/dataspace/parts/tests/p006/body.rs`, module `engine::dataspace::tests`):
     - `hegel_recorded_clock_responses_never_regress_or_exhaust_logical_time`: a recorded value is admitted exactly when it neither regresses below logical time nor reaches the unrestorable maximum, and the admitted snapshot restores.
     - `hegel_recorded_random_values_replay_only_inside_their_range`: a value replays exactly when it lies in `0..upper`.
     - `hegel_force_run_outranks_dependency_gated_in_every_order`: any order and mix of demands for one service derives force-run exactly when a force-run demand is present.
6. Replay, Wasm and Prolly:
   - Replay: `a_denied_demand_is_not_consumed_and_fails_the_step_even_when_ignored`, `a_transition_that_swallows_a_misbound_demand_error_fails_the_replay`, `the_trace_member_binds_the_trace_to_its_exact_record_bytes`, `equal_commit_ref_with_divergent_roots_is_a_root_divergence`, `equal_commit_ref_and_roots_with_field_differences_is_a_divergence`.
   - New Hegel property (`src/world_replay/tests/sealedproperties.rs`): `hegel_misbound_demands_never_consume_a_slot_and_always_fail_the_step` sends 0–3 request-ref or operation-id misbound demands. Each is counted as a violation, none consumes the slot, its true demand is still served, and the step finishes only when there were none.
   - Wasm: `wasm_nan_results_are_canonical_and_replay_identically` (mutation-checked above), `wasm_second_hostcall_is_denied_before_dispatch`.
   - Prolly: `a_byte_split_last_group_is_never_overfilled_by_borrowing_or_merging`, `rebalancing_still_borrows_and_merges_when_bytes_allow`, `exhaustive_small_group_shapes_respect_both_bounds_and_rebalance_maximally`, `byte_bounded_builds_never_exceed_max_node_bytes`.

**New properties fail on the old code (mutation check).** Each old behavior was reinstated temporarily: random values outside the range accepted, clock regressions accepted, dependency-gated outranking force-run, and a denied demand popping the next slot without counting a violation. All four properties failed with shrunk counterexamples: `recorded 1 for range 0..1`, `recorded 0 at logical time 1`, no force-run service derived, and zero violations counted. With the source restored they pass.

Docs:
- `docs/technical/foundations/canonical-identity-model.md`: the scoped discrepancy (the core codec accepted uppercase hex) now reads as a resolved discrepancy, and its "tracked as a defect" line is removed.
- The entry is removed from the discrepancy list in `docs/technical/README.md`.
- `python3 scripts/check-doc-links.py`: 320 files, 0 findings.

Commands (leviathan via `aremote`; formatting and Cairn locally):
- `cargo test -p molten-core --lib`: 823 passed, 0 failed.
- `cargo test -p molten --lib -- embedded optional_ref optional_content secret_optional serialized_identity closure_report_receipt chaoscontrol_fact engine::dataspace engine::logic testbed:: world_replay world_snapshot meshspace::recorded codec:: vault:: remote addressable_actor`: 409 passed, 0 failed (with the four new properties).
- `cargo test -p molten --test worldsnapshot --test fabricboundarycompat --test fabric_simulation_boundary`: 13 + 5 + 3 passed, 0 failed.
- `cargo test -p molten --lib --features world-snapshot-vm-cohort -- world_snapshot`: 24 passed. `cargo test -p molten --lib --features spacewasm-pilot -- spacewasm::`: 40 passed, 1 ignored.
- `cargo test -p molten --lib -- engine::dataspace`: 51 passed (includes the three new properties). `cargo test -p molten --lib -- runtime::dataspace world_replay`: 23 passed (includes the new sealed property).
- `cargo clippy -p molten -p molten-core -p molten-node-host --all-targets -- -D warnings`: exit 0, including the new property files.
- `cargo octet check --artifact-dir target/octet`: warning-only, 1053 findings workspace-wide (left for OctetSweep). None are in files this change touched or in the two new property files.
- Strict `cairn validate`: valid.

Review fix (FIX-FIRST): three `molten-core` boundaries still checked only the `blake3:` prefix, so they accepted uppercase, mixed-case and short digests:
- `committed_stream::commit_transition` (payload and state refs);
- `content_replication::check_publication`;
- `fabric_crypto_identity::artifact::auth::artifact_ref` (key, payload, verifier-context and currentness refs).

All three now use the shared lowercase validator `crate::fabric::valid_blake3_ref`.
- New negative regressions, each also accepting a lowercase digest: `committed_stream::tests::refs::only_lowercase_canonical_digests_commit`, `content_replication::tests::spelling::publication_admits_only_lowercase_canonical_digests`, `fabric_crypto_identity::tests::spelling::artifact_key_identity_admits_only_lowercase_canonical_digests`.
- Mutation check: with the shared validator temporarily made prefix-only (the old behavior), all three new tests failed. With the validator restored, they pass.
- A sweep of every `blake3:` prefix check in `crates/molten-core/src` found no other prefix-only acceptance. Every other site checks a 64-character lowercase hex digest, or only compares against such a digest (`artifact/auth.rs` identity drift).
- `cargo test -p molten-core --lib`: 826 passed. `cargo test -p molten --test content_replication`: 12 passed. Clippy (`-p molten -p molten-core -p molten-node-host --all-targets -D warnings`): exit 0.
- `docs/technical/README.md` keeps a canonical-identity entry, and the article records the fix as tracked, until it is merged.
- **Final task unchecked** until this fix lands on `molten`.
