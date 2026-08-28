## Phase 1: Promotion and release core

- [x] [depends:add-world-branch-head-protocol] [depends:adopt-artifact-binding-and-semantic-effects] Record baseline head, effect-log, durable-state, delivery-idempotency, and reconciliation checks. r[molten.world_promotion.verification]
- [x] [serial] Define promotion plan, intent closure, release reservation, lifecycle state, operation identity, observation, reconciliation, and diagnostic DTOs. r[molten.world_promotion.plan] r[molten.world_promotion.dispatch]
- [x] [depends:world-promotion-dtos] Implement pure promotion admission, stable release identity, complete reservation-set checks, and exact state transitions. r[molten.world_promotion.plan] r[molten.world_promotion.transaction]
- [x] [depends:transactional-reconciliation-core-publication] Map uncertain local publication observations into Transactional Reconciliation Core without transferring Molten mutation authority. r[molten.world_promotion.reconciliation]
- [x] [parallel] Add canonical Preserves promotion, reservation, attempt, observation, and reconciliation schemas. r[molten.world_promotion.plan] r[molten.world_promotion.dispatch]

## Phase 2: Atomic local shell and dispatcher

- [x] [depends:world-promotion-core] Add narrow active-head, transaction, reservation, dispatcher, current-authority, effect-admission, observation, and reconciliation ports. r[molten.world_promotion.transaction] r[molten.world_promotion.dispatch]
- [x] [depends:world-promotion-ports] Implement one local transaction that rechecks state and atomically publishes the active head with the complete reservation set. r[molten.world_promotion.transaction]
- [x] [depends:world-promotion-local-transaction] Implement reservation claim, current admission recheck, adapter dispatch, attempt recording, and acknowledgment ingestion. r[molten.world_promotion.dispatch]
- [x] [depends:world-promotion-local-transaction] Implement observation-first recovery for unknown, conflicting, duplicate, and partially observed outcomes. r[molten.world_promotion.reconciliation]
- [x] [depends:world-promotion-dispatch] Add operator plan, promote, outbox-inspect, retry-plan, reconcile, deny, and abandon commands with fail-closed standalone mutation. r[molten.world_promotion.non_claims]

## Phase 3: Verification and documentation

- [x] [parallel] Add positive atomic promotion, complete reservation, safe retry, denied-after-promotion, and follow-up observation-commit fixtures. r[molten.world_promotion.verification]
  - Verified: atomic promotion, complete reservations, safe retry, denied-after-promotion, receipt-last sidecars, ordered effect-log parity, and one recorded-effect successor.
- [x] [parallel] Add negative stale head, incomplete intent closure, reservation mismatch, simulated branch, denied capability, crash before commit, unknown commit, crash after dispatch, lost acknowledgment, duplicate response, conflicting observation, and exactly-once-overclaim fixtures. r[molten.world_promotion.verification]
  - Verified: stale head, incomplete closure, reservation mismatch, simulation denial, capability denial, unknown commit/readback, lost acknowledgment, duplicate-risk acknowledgment, conflict, overclaim denial, missing outcome, mismatched outcome, live fallback, uncommitted reservation, unacknowledged observation, and unchanged successor.
- [x] [serial] Document eligibility atomicity, external completion limits, lifecycle states, adapter idempotency profiles, and reconciliation procedures. r[molten.world_promotion.non_claims]
- [x] [depends:world-promotion-verification] Run focused tests, Molten effect-log and reconciliation compatibility fixtures, Octet, Clippy with warnings denied, Cairn gates, lifecycle checks, and relevant Nix checks. r[molten.world_promotion.verification]
  - Passed: focused and full workspace tests, strict Clippy, zero-finding Octet, deterministic plans, focused Nix checks, Nix evaluation, and Cairn gates.
  - Bounded: the inherited `contract-export-drift-gate` remains outside this change and still fails.
