# Promotion and reservation atomicity

Promotion joins two local facts: a candidate becomes the active world, and its complete release-reservation set becomes eligible for later dispatch. This article assumes familiarity with world commits and branch generations. It explains the persistence boundary, not a distributed transaction with an external service. The [promotion contract](../../world-promotion-and-effect-release.md) remains authoritative; the [Technical companion](../README.md) places this discussion among the other world mechanisms.

## Separate immutable intent from mutable execution

A candidate commit contains immutable effect intents. Promotion records, reservations, attempts, and observations live outside that commit. This separation allows the same candidate identity to survive changes in operational knowledge without rewriting history. A reservation binds a particular promotion, candidate, intent, semantic operation, handler, adapter, and successor generation. An attempt is a particular effort to discharge that reservation, not a replacement identity for the intent.

Consequently, three statements have different meanings:

- The candidate describes an effect that may become eligible.
- A committed reservation records local eligibility under a promotion.
- An attempt observation records what the shell learned from an adapter interaction.

None of these statements alone establishes external completion. The [dispatch service](../../../src/world_promotion/service.rs) makes the distinction concrete: it persists an attempting reservation and attempt before calling the adapter, then stores the classified observation afterward. Each of those steps writes the reservation and its attempt record in one Redb write transaction (`record_attempt_transition`), so a crash can never leave the reservation `attempting` without its attempt record or an attempt's result without its reservation's. `recover_interrupted_attempts` classifies an interrupted `attempting` pair as uncertain, and also settles states that stores written before this coupling may still hold: an `attempting` reservation without an attempt row gains an uncertain attempt at its initial attempt identity, one whose attempt already recorded a result takes that result's state, and an abandoned attempt whose reservation stayed `uncertain` (or `attempting`) brings that reservation to `abandoned`.

## What the transaction actually couples

`LocalWorldPromotionStore::commit_promotion` parses committed reservation records, checks their reference set against the plan, and opens a Redb write transaction. Inside that transaction it reads the live branch head and whether the promote grant's nonce is already consumed, and runs `validate_promotion_transaction(plan, facts, live)` against those transactional observations; only then does it write the successor head, canonical promotion record, reservations, operation identity, and consumed grant nonce. Only the final transaction commit publishes these writes together. See the [transaction implementation](../../../src/world_promotion/store/transaction.rs).

The shell-supplied facts carry the signed grant, which must authorize exactly `promote` of this plan's candidate head at the predecessor generation, be admitted and unexpired, and carry an unconsumed nonce; the grant, closure and material checks no longer depend on a head observed before the transaction. A head that differs from the planned predecessor denies the transaction rather than being overwritten. When the operation's identity is already recorded, the store reports `Applied` only if the successor head and every planned reservation (compared by identity fields, since their states may have advanced) are present, and `Inconsistent` otherwise, without writing or consuming authority.

Operator mutations follow the same rule. Deny, retry planning, and abandon decide their action- and target-bound grant against the live head and nonce, then `apply_operator_mutation` rechecks that head and nonce in the write transaction that writes the reservation and attempt changes and consumes the nonce. Abandonment moves both the uncertain attempt and its reservation to `abandoned` in that transaction. `update_reservation` and every combined transition reject changes to a reservation's promotion, operation, candidate, intent, semantic, handler, or adapter reference or its generation.

## Unknown publication is a distinct result

A failure returned by the final Redb commit maps to `OutcomeUnknown`, not to definite non-publication. This is an epistemic distinction: an error reporting publication does not necessarily identify the durable state that a later observer will find.

`promote_world` classifies the commit observation and invokes read-back reconciliation when quarantine is not clear. The resulting persistence receipt is published after classification. The closed persistence meanings described by the [contract](../../world-promotion-and-effect-release.md) are not-published, published, publication-unknown, and conflicting. They prevent callers from collapsing uncertainty into either a successful effect or a safe automatic retry.

Read-back is itself bounded evidence. The store reads the head, the promotion record, and the expected reservation entries. Because they are published together, any partial subset is `Corrupt`: the successor head without every reservation or without the promotion record, or reservations and the record without the successor head. A promoted plan whose head was later superseded on the same branch still reads back as applied. Reservations count as present when their identity fields match the plan, in any post-commit state, so idempotent replay keeps classifying a promotion as published after its reservations were claimed, attempted, denied, or abandoned. Reviewers should inspect the precise equality checks rather than treating a receipt label as a stronger database audit than the implementation performs.

## Illustrative lost-acknowledgment scenario

Suppose a candidate has two intents, A and B, and replaces generation 12 with generation 13. These labels and numbers are illustrative, not serialized references.

1. Promotion commits the generation-13 head and both committed reservations in one local transaction.
2. Dispatch claims A, observes current admission, and persists an attempting record.
3. The external adapter may perform A, but its acknowledgment is lost.
4. B remains a distinct reservation; the missing acknowledgment for A cannot be converted into an outcome for either intent.

The uncertainty belongs to A's attempt. An operator-authorized retry needs a new attempt identity and explicit duplicate-risk acknowledgment while retaining A's reservation identity. Neither the original transaction nor retry bookkeeping creates an exactly-once guarantee. Abandonment likewise records acknowledged uncertainty rather than manufacturing a negative external result. A later logical recorded-effect successor needs an acknowledged, correctly bound observation under the [replay boundary](../../world-replay-capsules.md).

## Verification and limits

Suggested review, not executed evidence: inspect fault cases at the transaction boundary, stale predecessor comparisons, incomplete already-successor reservation sets, and failure after adapter invocation but before observation persistence. At the dispatch boundary, verify that denied current admission stores a blocked reservation without calling the adapter. Distinguish these cases from a missing adapter or a receipt-publication failure.

The standalone promotion CLI deliberately lacks ambient current-authority composition. Planning and outbox inspection therefore do not demonstrate live dispatch readiness. Local atomicity establishes a coupled eligibility transition; it does not establish external atomicity, current authority forever, adapter correctness, or release safety for an arbitrary deployment.

## Sources

- [World promotion and effect release](../../world-promotion-and-effect-release.md)
- [World replay capsules](../../world-replay-capsules.md)
- [World operator workflows](../../world-operator-workflows.md)
- [Promotion transaction and read-back](../../../src/world_promotion/store/transaction.rs)
- [Promotion and dispatch orchestration](../../../src/world_promotion/service.rs)
- [Atomic reservation and attempt transitions](../../../src/world_promotion/store/outbox.rs)
- [Interrupted-attempt recovery](../../../src/world_promotion/recovery/mod.rs)
- [Operator mutations](../../../src/world_promotion/operator/mod.rs)
- [Technical companion](../README.md)
