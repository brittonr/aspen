## Phase 1: Promotion and release core

- [ ] [depends:add-world-branch-head-protocol] [depends:adopt-artifact-binding-and-semantic-effects] Record baseline head, effect-log, durable-state, delivery-idempotency, and reconciliation checks. r[molten.world_promotion.verification]
- [ ] [serial] Define promotion plan, intent closure, release reservation, lifecycle state, operation identity, observation, reconciliation, and diagnostic DTOs. r[molten.world_promotion.plan] r[molten.world_promotion.dispatch]
- [ ] [depends:world-promotion-dtos] Implement pure promotion admission, stable release identity, complete reservation-set checks, and exact state transitions. r[molten.world_promotion.plan] r[molten.world_promotion.transaction]
- [ ] [depends:transactional-reconciliation-core-publication] Map uncertain local publication observations into Transactional Reconciliation Core without transferring Molten mutation authority. r[molten.world_promotion.reconciliation]
- [ ] [parallel] Add canonical Preserves promotion, reservation, attempt, observation, and reconciliation schemas. r[molten.world_promotion.plan] r[molten.world_promotion.dispatch]

## Phase 2: Atomic local shell and dispatcher

- [ ] [depends:world-promotion-core] Add narrow active-head, transaction, reservation, dispatcher, current-authority, effect-admission, observation, and reconciliation ports. r[molten.world_promotion.transaction] r[molten.world_promotion.dispatch]
- [ ] [depends:world-promotion-ports] Implement one local transaction that rechecks state and atomically publishes the active head with the complete reservation set. r[molten.world_promotion.transaction]
- [ ] [depends:world-promotion-local-transaction] Implement reservation claim, current admission recheck, adapter dispatch, attempt recording, and acknowledgment ingestion. r[molten.world_promotion.dispatch]
- [ ] [depends:world-promotion-local-transaction] Implement observation-first recovery for unknown, conflicting, duplicate, and partially observed outcomes. r[molten.world_promotion.reconciliation]
- [ ] [depends:world-promotion-dispatch] Add operator plan, promote, outbox-inspect, retry-plan, reconcile, deny, and abandon commands. r[molten.world_promotion.non_claims]

## Phase 3: Verification and documentation

- [ ] [parallel] Add positive atomic promotion, complete reservation, safe retry, denied-after-promotion, and follow-up observation-commit fixtures. r[molten.world_promotion.verification]
- [ ] [parallel] Add negative stale head, incomplete intent closure, reservation mismatch, simulated branch, denied capability, crash before commit, unknown commit, crash after dispatch, lost acknowledgment, duplicate response, conflicting observation, and exactly-once-overclaim fixtures. r[molten.world_promotion.verification]
- [ ] [serial] Document eligibility atomicity, external completion limits, lifecycle states, adapter idempotency profiles, and reconciliation procedures. r[molten.world_promotion.non_claims]
- [ ] [depends:world-promotion-verification] Run focused tests, Weft and reconciliation compatibility fixtures, Octet, Clippy with warnings denied, Cairn gates, lifecycle checks, and relevant Nix checks. r[molten.world_promotion.verification]
