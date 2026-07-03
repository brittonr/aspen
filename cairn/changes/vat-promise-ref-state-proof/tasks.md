# Tasks: vat-promise-ref-state-proof

## Phase 1: Predicate proof surface

- [ ] [serial] r[molten.vat_ref_state_proof.promise_lifecycle] Define pure promise/vow transition checks for pending, resolved, broken, cancelled, timed-out, and causal-failure states.
- [ ] [parallel] r[molten.vat_ref_state_proof.reference_lifetime] Define pure near/far/distributed ref lifetime checks for locality, handoff admission, stale use, and revocation.
- [ ] [parallel] r[molten.vat_ref_state_proof.rollback_cleanup] Add rollback and cleanup checks for actormap transactions, assertions, observers, pending calls, and authority snapshots.

## Phase 2: Tests

- [ ] [parallel] r[molten.vat_ref_state_proof.promise_lifecycle] Add positive pending→resolved and pending→broken tests plus unresolved pipeline denial tests.
- [ ] [parallel] r[molten.vat_ref_state_proof.reference_lifetime] Add legal near/far routing, admitted handoff, stale far ref denial, and synchronous far-call denial tests.
- [ ] [parallel] r[molten.vat_ref_state_proof.rollback_cleanup] Add transaction rollback leak tests, revocation cleanup tests, and no-authority-amplification tests.

## Phase 3: Evidence and validation

- [ ] [serial] r[molten.vat_ref_state_proof.promise_lifecycle] r[molten.vat_ref_state_proof.reference_lifetime] r[molten.vat_ref_state_proof.rollback_cleanup] Bind proof refs and run `cargo test vat runtime`.
