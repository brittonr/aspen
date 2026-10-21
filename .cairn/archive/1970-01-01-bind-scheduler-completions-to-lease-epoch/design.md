# Design: Bind scheduler completions to the lease epoch

## Context

`crates/molten-core/src/fabric_simulation/reference.rs` owns the reference scheduler transition. `DistributedSchedulerOperation::Complete { job_id, owner, completion_ref }` cannot check an expected epoch because the request carries none. `Lease` and `Failover` both increment `lease_epoch`, so the epoch already distinguishes successive assignments of the same job to the same owner.

## Approach

- Extend `Complete` with an `assignment_token` field. `Lease` derives the token from a canonical domain-separated encoding of job id, owner, and lease epoch, hashed with BLAKE3. The state machine recomputes the token from current state at completion time and compares exact equality.
- Canonical encoding uses the same delimiter discipline as existing reference refs; the token is opaque to workers, so no encoding is exposed beyond the lease response.
- Rejection returns a new typed issue, `StaleAssignmentToken`, distinct from `LeaseOwnerMismatch`, so diagnostics name the actual defect.
- The owner-returns regression runs through the pure transition function: submit, lease A, failover B, failover A, complete with A's epoch-1 token; the call must return `StaleAssignmentToken`.
- The independent check inspects the transition history, not the service report: it derives the authoritative-completion set from recorded lease and failover events and fails when a completion references an epoch other than the assignment that owned the job at that logical position. A deliberately broken transition fixture (token check removed) must fail this check while reporting its own invariants as passed.

## Alternatives considered

- Add `expected_epoch: u64` instead of a token. Simpler, but does not bind the completion to the assignment content and diverges from the coordination-delivery token design that this change reuses as its reference.
- Fix only the fixture checker. Rejected: the state machine defect is real and cheap to correct.

## Non-claims

- This change covers the reference scheduler only. It does not claim that production scheduler or delivery paths contain this defect.
- Independent-check agreement does not prove whole-system correctness.

## Risks

- Operation shape change breaks fixtures that construct `Complete`; all call sites in `tests.rs` and `reference.rs` fixtures update in the same change.
