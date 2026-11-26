# Design

## Time Reservations, Batch Atomicity, and Fenced Lease Renewal

- In `crates/molten-core/src/fabric_time/reservation/batch.rs`:
  - Permit `expire_reservation` when `state` is `Committed` or `Stale` (rejecting already `Released`), so delayed `ObservedStop` or `FencingTakeover` enforcement can always transition `Stale -> Released`.
  - On `Release`, restore both `next.pool.{reserved,allocated}_{units,bytes}` AND `next.workers[worker_idx].{units,bytes}` (with a conservation invariant check that pool and worker totals reconcile).
- In `src/fabric_time/shell/reservation.rs`:
  - Commit each admitted batch item to `self.state` immediately as its external allocation succeeds (or persist intent before allocation and transition per item), preserving `FabricPortError::Uncertain` rather than flattening into `Failure::invalid_harness`.
- In `crates/molten-core/src/fabric_time/lease.rs`:
  - Route `(LeaseAction::Renew, DeadlineStatus::Pending)` through `classify_exclusive_lease` (or an explicit renewal fencing check) so `FencedExclusive` renewals with missing or non-advancing/stale fencing tokens (`fencing_token < previous_fencing_token` or missing token) are denied.

## Recoverable Membership Slots, Healthy-First Placement, and Deterministic Observation Reduction

- In `src/fabric_membership/{ports.rs,shell.rs,persistence.rs,persistence/codec.rs}`:
  - Add `inspect_pending`, `abort_pre_effect_intent` (admitted only with definite pre-effect non-execution evidence), and `reconcile_pending_intent` (for post-effect uncertainty with operator/fencing evidence) to `AssignmentPersistence` and `Redb`.
- In `crates/molten-core/src/fabric_membership/transition.rs` and `src/fabric_membership/persistence.rs`:
  - Allow `validate_assignment_successor` to admit a strictly higher-epoch/fenced successor over a `Quarantined` predecessor when explicit quarantine-clearance/fencing evidence is supplied (or provide an explicit fenced `Quarantined -> Released` transition), eliminating permanent role-slot bricking.
- In `crates/molten-core/src/fabric_membership/mod.rs`:
  - Sort eligible placement candidates by `(health_tier ASC [Healthy before Suspected/Unavailable], preference_score DESC, tie_rank ASC, node_id ASC)` so `allow_degraded` acts strictly as a fallback when insufficient healthy nodes exist.
  - Break failure-observation ties deterministically by `detector_profile_ref` (eliminating input-slice order dependence) and reject or partition comparisons across distinct `time_basis_ref` values rather than comparing incompatible tick epochs.

## Truthful Coordination Tokens, Assertion Refs, and Lock/Election Expiry

- In `src/coordination/parts/mod/{p003,p004,p005}/body.rs`:
  - Compute the real `FencingToken` bound to `commit_receipt_ref` and commit that exact `token_ref` into `LockState`/`ElectionState` rather than leaving `fixture_ref("pending-coordination-commit")` in persisted state.
  - Break the circular receipt/assertion placeholder hash cleanly (e.g., bind `CoordinationStatusAssertion` to `transition_ref`/`commit_receipt_ref` and store the actual emitted assertion ref in the receipt, or separate status-assertion identity from the receipt envelope without hashing discarded placeholders).
  - Add explicit bounded lease-expiry transitions for locks (`LockExpire`) and explicit resignation/expiry transitions for elections (`ElectionResign` / `ElectionExpire`) so crashed holders cannot permanently wedge coordination keys.
  - In `p002/body.rs`, validate and bind the caller's `control_group_ref` explicitly rather than silently substituting `control_registry_fixture_manifest_value()`.
