## Why

Audit of Aspen's canonical Preserves encoding, BLAKE3 identity derivation, Runtime Dataspace state machine, World Replay, Fabric Simulation, Wasm harness, and Prolly Map uncovered direct violations of Aspen's two non-negotiable foundational invariants (Canonical Identity and Deterministic Playback):

1. **Constructible BLAKE3 hash collision (`crates/molten-core/src/fabric_simulation/live_differential.rs:201-230`):** `valid_ordering_key` permits `\0` bytes, and `workload_ref` concatenates fields with raw `\0` delimiters without length framing or Preserves encoding—allowing a valid 1-step workload with an embedded `\0...` payload in `ordering_key` to collide identically with a distinct 2-step workload. Similarly, `failure_fingerprint` (`crates/molten-core/src/fabric_simulation/replay.rs:83-107`) hashes unframed `|`-joined strings without `.dedup()`.
2. **Preserves `Embedded` values bypass all security, secret, and GC structural scans (`src/preserves/parts/rail/p008/body.rs:151-217`):** `strict_canonical_decode` accepts canonical packed `Embedded` values (`#:<...>`), but `container_children` returns `Vec::new()` for `Embedded`, letting wrapped `<host-path>`, `<command>`, `<secret>`, or `"blake3:..."` references completely evade ambient-token, sensitive-marker, and content-ref scans.
3. **`serde_json::to_vec` and Rust `Debug` output hashed directly into canonical BLAKE3 identities:** `src/world_snapshot/chaoscontrol.rs:85-211` and `src/world_snapshot/vm/helpers.rs:31-167` hash `serde_json::to_vec` into canonical cohort/plan/artifact identities; `src/worldcommit/records.rs:75,191`, `crates/molten-core/src/worldcommit/restore.rs:36,117`, and `src/world_head/parts/service/p001/body.rs:57` serialize `format!("{issue:?}")` directly into canonical Preserves receipts and hash them into `receipt_ref`/`report_ref`.
4. **Canonical encoding/decoding asymmetries:** `optional_content_ref` / `parse_optional_ref` (`src/preserves/parts/rail/{p003,p006}/body.rs`, `src/secrets/parts/mod/p006/body.rs`) accept both `<some "blake3:...">` and bare `"blake3:..."` as `Some(ContentRef)` (two canonical digests for one logical struct), and 8 validators across `crates/molten-core` and `src/addressable_actor/ports.rs` accept uppercase hex digests while `ContentRef::parse` requires strict lowercase.
5. **Runtime Dataspace state-machine bugs (`src/runtime/dataspace/parts/state/{p000,p001,p002,p003,p004}/body.rs`):**
   - `recorded_effect_response_transition` allows `logical_time` regression and unchecked `value + 1` overflow, and accepts arbitrary recorded `Effect::Random` values without checking `value < *upper`.
   - Live `local_clock_response_value` / `next_effect_sequence` can reach un-restorable `u64::MAX` and overflow.
   - Rolled-back retractions emit `Event::TurnRolledBack` *and* `Event::TurnCommitted` with a `Pass` predicate receipt.
   - All five `apply_facet_*` turns commit with empty `actor: ""`, and `stage_single_facet_stop` fails to emit `Event::AssertionRetractionObserved`.
   - `ServiceDemandSet::derive` silently drops `ForceRun` behind `DependencyGated` because `"dependency-gated" < "force-run"`.
6. **World Replay, Fabric Simulation, Wasm determinism & Prolly Map bugs:**
   - `SealedReplayObservations::observe` (`src/world_replay/boundary/sealed.rs:70-80`) removes the pending observation *before* checking binding without recording a violation, letting mismatched demands be silently swallowed.
   - `world_replay_trace_member` (`src/world_replay/adapters.rs:12-26`) never binds `trace` to `record` bytes; `compare_step` (`crates/molten-core/src/world_replay/comparison.rs:68-70`) short-circuits on `commit_ref` alone without checking `roots` or `field_differences`.
   - `evaluate_live_differential` (`live_differential.rs:258-301`) never validates `simulated_class`, and `compare_replay` (`replay.rs:53-76`) ignores `selected.{node_id,kind,ready_at_tick}` and eligible choice metadata.
   - Wasm harness (`src/harness/wasm/parts/executor/p000/body.rs:218-251`) omits `cranelift_nan_canonicalization(true)` and enforces `require_single_call` post-hoc.
   - Prolly Map `rebalance_last_group` (`crates/molten-core/src/prolly_map/tree/build.rs:204-226`) merges byte-split internal nodes checking only `max_fanout` and never `max_node_bytes`, causing `NodeSizeLimitExceeded`.

## What Changes

- Replace raw delimiter concatenation in `LiveWorkload::workload_ref` and `failure_fingerprint` with canonical packed Preserves encoding (with domain separation and `.dedup()`), and reject NUL/control characters in `ordering_key`.
- Descend into Preserves `Embedded` values inside `container_children` / `inspect_structural_node` (or reject `Embedded` at security-scanned boundaries).
- Replace all `serde_json::to_vec` and `format!("{issue:?}")` identity/receipt encodings with canonical packed Preserves and closed typed issue codes.
- Eliminate dual `Option<ContentRef>` encodings and enforce lowercase-only `blake3:` hex validation uniformly across `crates/molten-core` and `src/`.
- Repair all Runtime Dataspace, World Replay, Fabric Simulation, Wasm NaN/hostcall determinism, and Prolly Map `rebalance_last_group` bugs listed above.

## Impact

- **Files**: `crates/molten-core/src/fabric_simulation/{live_differential.rs,replay.rs}`, `src/preserves/parts/rail/{p003,p006,p008}/body.rs`, `src/secrets/parts/mod/p006/body.rs`, `src/world_snapshot/{chaoscontrol.rs,vm/helpers.rs}`, `src/worldcommit/{records.rs,shell.rs}`, `crates/molten-core/src/worldcommit/restore.rs`, `src/world_head/parts/service/p001/body.rs`, `crates/molten-core/src/{preserves_profile.rs,codec.rs,stack.rs,fabric_time/mod.rs,fabric_time/reservation/batch.rs,fabric_simulation/admission.rs,world_witness/reconciliation.rs}`, `src/addressable_actor/ports.rs`, `src/runtime/dataspace/parts/state/{p000..p004}/body.rs`, `src/world_replay/{boundary/sealed.rs,adapters.rs}`, `crates/molten-core/src/world_replay/comparison.rs`, `src/harness/wasm/parts/executor/p000/body.rs`, and `crates/molten-core/src/prolly_map/tree/build.rs`.
- **Verification**: Regression and Hegel property tests reproducing the `workload_ref` collision, `Embedded` bypass, uppercase/dual-option rejection, rolled-back retraction receipt, `logical_time` regression/overflow, `ForceRun` shadowing, `SealedReplayObservations` pop-before-validate, `compare_step` root mismatch, and Prolly Map byte-split rebalance overflow.
- **Non-claims**: Closing these canonical-encoding and determinism bugs restores single-node deterministic playback and collision resistance; it does not certify external guest VM determinism.

## Out of Scope

- World promotion / world head / world merge authority and transaction fixes (tracked in `harden-world-promotion-head-and-merge-semantics`).
- Steel VM sandboxing (tracked in `fix-authority-bypasses-and-sandbox-isolation`).
