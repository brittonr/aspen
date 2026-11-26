# Design

## Action-Bound Canonical Promotion Authority

- Extend `SignedPromotionAuthority` (`src/world_promotion/current.rs`) with explicit closed `action` (`Promote`, `DenyReservation`, `PlanRetry`, `AbandonUncertain`), exact target binding (`candidate_head` for `Promote`, `reservation_ref`/`attempt_ref` for operator actions), unique nonce/expiry, and canonical packed Preserves encoding for Ed25519 signing (eliminating `\n`/`,` string concatenation in `authority_message` and `closure_message`).
- In `src/world_promotion/operator/mod.rs`, read the live branch head from `WORLD_HEADS_TABLE` and verify that operator authority matches the current branch generation (or explicitly authorized historical-reconciliation policy) rather than trusting stale historical generation grants indefinitely.

## Atomic Promotion & Dispatch Transactions

- In `src/world_promotion/service.rs` and `store/outbox.rs`:
  - Merge `update_reservation(Attempting)` + `store_attempt(Attempting)` into one Redb write transaction `begin_attempt_atomic`, and merge `store_attempt(terminal)` + `update_reservation(terminal)` into one Redb write transaction `finish_attempt_atomic`.
  - Add recovery handling for any legacy/interrupted `Attempting` reservation without a matching in-progress attempt row.
  - Update `abandon_uncertain_for_attempt` (`operator/mod.rs`) to transition both the attempt record and its parent reservation atomically to `Abandoned`.
  - Assert immutability of `candidate_head`, `intent_ref`, `handler_ref`, `adapter_ref`, and `generation` inside `update_reservation`.
- In `src/world_promotion/store/transaction.rs`:
  - Move `validate_promotion_transaction` inside the Redb write transaction (`begin_write()`) against live transactional head/fact observations.
  - Fix `read_back_promotion` so `head == Some(&plan.after)` with `present < plan.reservations.len()` always returns `Corrupt` (never falling through to `Prior`), and distinguish valid post-promotion reservation states (`Claimed`/`Attempting`/`Observed`/`Uncertain`/`Denied`/`Abandoned`) during idempotent replay checks from corrupt records.

## Truthful World Head Receipts and Verified Signer Roles

- In `src/world_head/parts/service/p000/body.rs`: build the returned canonical receipt *after* `store.apply_transition` using the actual `WorldHeadMutationOutcome` (`applied`, `stale`, or `uncertain`) so CAS conflicts and uncertain commits never return or persist `decision: "admitted"`.
- In `src/world_head/parts/service/p001/body.rs`: derive `role` from the verified signed `ArtifactStatement` / `AuthenticationPolicy.trusted_keys` and derive `authority_admitted` strictly from `WorldHeadAuthorityPort` rather than copying unsigned `carrier` fields.

## Conservative World Merge and Complete Closure Graph Validation

- In `crates/molten-core/src/world_merge/admission.rs`: evaluate `is_runtime_sensitive(root.kind)`, availability (`validate_schema_admission` / `available`), and `request.profile.root_modes.get(&root.kind)` *before* any equal-root shortcut.
- In `crates/molten-core/src/world_merge/planning/application.rs`: verify `profile.input_schema` against `base`, `left`, and `right` root schema references before invoking an application handler.
- In `crates/molten-core/src/world_merge/diff.rs`: classify `base = None` with differing `left`/`right` (or one-sided creation/deletion) as `Changed` or `Incompatible`, reserving `Absent` for all-three-absent roots.
- In `crates/molten-core/src/worldcommit/restore.rs`: run cycle and reachability checks across *every* node in `request.parent_graph`, rejecting unreachable orphan commits and detached cycles.
