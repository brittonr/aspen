# Design

## Canonical Preserves & BLAKE3 Identity Integrity

- **Collision-free `workload_ref` and `failure_fingerprint` (`crates/molten-core/src/fabric_simulation/{live_differential.rs,replay.rs}`):** Reject control/NUL bytes in `valid_ordering_key`, serialize workload steps and failure fingerprints as domain-tagged canonical packed Preserves (or length-prefixed canonical structures inside `molten-core`), and `.dedup()` `failed_invariants` before hashing.
- **Preserves `Embedded` structural traversal (`src/preserves/parts/rail/p008/body.rs`):** In `container_children`, check `value.is_embedded()` / `value.as_embedded()` and emit the inner value into the traversal queue (respecting depth/node budgets) so `find_ambient_job_token`, `find_sensitive_structural_marker`, and `contains_structural_content_ref` inspect embedded payloads identically to records/sequences.
- **Eliminate `serde_json` and `Debug` hashing:**
  - Replace `json_identity` / `json_ref` in `src/world_snapshot/chaoscontrol.rs` and `vm/helpers.rs` with canonical packed Preserves projection + BLAKE3.
  - Replace `format!("{issue:?}")` in `src/worldcommit/{records.rs,shell.rs}`, `crates/molten-core/src/worldcommit/restore.rs`, and `src/world_head/parts/service/p001/body.rs` with stable closed canonical issue codes/records.
- **Unique canonical representation:** Require `<some "blake3:...">` / `<none>` strictly in `optional_content_ref` and `parse_optional_ref` (rejecting bare `"blake3:..."` ambiguity), and enforce lowercase-only `0..=9 | a..=f` across all `molten-core` and `addressable_actor` `blake3:` validators.

## Runtime Dataspace & Wasm Determinism Repairs

- In `src/runtime/dataspace/parts/state/`:
  - `p000` & `p001`: Use checked arithmetic (`checked_add(1)` rejecting `u64::MAX`), enforce monotonicity (`value >= before.logical_time`) on recorded clock transitions, and verify `value < *upper` on recorded `Effect::Random`.
  - `p003`: Do not invoke `commit_turn_with_predicate_receipt` / emit `Event::TurnCommitted` + `PredicateDecision::Pass` when a retraction rolled back; return a rolled-back outcome and receipt cleanly.
  - `p002`: Pass the actual owning actor ID into `apply_facet_*` turns (never `""`) and emit `Event::AssertionRetractionObserved` for observers matching assertions retracted during `stage_single_facet_stop`.
  - `p004`: Give `ForceRun` explicit precedence over `DependencyGated` in `ServiceDemandSet::derive` instead of lexicographical first-match shadowing.
- In `src/harness/wasm/parts/executor/p000/body.rs`: enable `config.cranelift_nan_canonicalization(true)` and enforce `require_single_call` synchronously inside the hostcall trampoline before executing a second call.

## World Replay, Simulation Comparison, and Prolly Map Repairs

- `src/world_replay/boundary/sealed.rs`: Peek/validate `slot.{source_id,operation_id,request_ref}` *before* removing from `self.pending`, and increment a violation counter on mismatched demands so `finish_step` fails closed.
- `src/world_replay/adapters.rs`: Verify `record.bytes` decodes to `trace` and matches `trace.trace_ref` before pairing them in `world_replay_trace_member`.
- `crates/molten-core/src/world_replay/comparison.rs`: Always verify `observation.actual.roots` against `expected_commit.roots` and reject non-empty `observation.field_differences` even when `commit_ref == step.expected_successor`.
- `crates/molten-core/src/fabric_simulation/{live_differential.rs,replay.rs}`: Validate `simulated_class` against `Unsupported` and `declared.permitted` in `evaluate_live_differential`, and compare `selected.{node_id,kind,ready_at_tick}` plus full eligible-choice metadata in `compare_replay`.
- `crates/molten-core/src/prolly_map/tree/build.rs`: Check `internal_encoded_len(profile, ...) <= profile.max_node_bytes` before merging or borrowing in `rebalance_last_group` so byte-bounded splits never overflow `max_node_bytes`.
