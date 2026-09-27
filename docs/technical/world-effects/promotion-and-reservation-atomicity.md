# Promotion and reservation atomicity

Promotion joins two local facts: a candidate becomes the active world, and its complete release-reservation set becomes eligible for later dispatch. This article assumes familiarity with world commits and branch generations. It explains the persistence boundary, not a distributed transaction with an external service. The [promotion contract](../../world-promotion-and-effect-release.md) remains authoritative; the [Technical companion](../README.md) places this discussion among the other world mechanisms.

## Separate immutable intent from mutable execution

A candidate commit contains immutable effect intents. Promotion records, reservations, attempts, and observations live outside that commit. This separation allows the same candidate identity to survive changes in operational knowledge without rewriting history. A reservation binds a particular promotion, candidate, intent, semantic operation, handler, adapter, and successor generation. An attempt is a particular effort to discharge that reservation, not a replacement identity for the intent.

Consequently, three statements have different meanings:

- The candidate describes an effect that may become eligible.
- A committed reservation records local eligibility under a promotion.
- An attempt observation records what the shell learned from an adapter interaction.

None of these statements alone establishes external completion. The [dispatch service](../../../src/world_promotion/service.rs) makes the distinction concrete: it persists an attempting reservation and attempt before calling the adapter, then stores the classified observation afterward.

## What the transaction actually couples

`LocalWorldPromotionStore::commit_promotion` parses committed reservation records, checks their reference set against the plan, and opens a Redb write transaction. Inside that transaction it reads the branch head, compares it with the planned predecessor, and writes the successor head, canonical promotion record, and reservations. Only the final transaction commit publishes these writes together. See the [transaction implementation](../../../src/world_promotion/store/transaction.rs).

The already-successor case is not treated as unconditional success. The store checks whether the expected reservation records match; an incomplete set produces `Inconsistent`. A different current head produces a non-applied observation rather than overwriting the competing state. This preserves the distinction between repeating a known local operation and authorizing a new transition.

There is an important source boundary. The governing document describes rechecking current transaction facts within the atomic sequence. The inspected implementation calls `validate_promotion_transaction(plan, facts)` before `begin_write`, while the head comparison occurs inside the write transaction. The shell obtains those supplied facts through `observe_transaction` before calling the store. Thus the source establishes atomic head-and-reservation publication, but does not establish that every authority or policy observation is acquired inside the Redb transaction. This article does not resolve that wording difference by assuming an uninspected adapter guarantee.

## Unknown publication is a distinct result

A failure returned by the final Redb commit maps to `OutcomeUnknown`, not to definite non-publication. This is an epistemic distinction: an error reporting publication does not necessarily identify the durable state that a later observer will find.

`promote_world` classifies the commit observation and invokes read-back reconciliation when quarantine is not clear. The resulting persistence receipt is published after classification. The closed persistence meanings described by the [contract](../../world-promotion-and-effect-release.md) are not-published, published, publication-unknown, and conflicting. They prevent callers from collapsing uncertainty into either a successful effect or a safe automatic retry.

Read-back is itself bounded evidence. The inspected store reads the head and expected reservation entries; corrupt records and missing state have explicit observations. Reviewers should inspect the precise equality checks rather than treating a receipt label as a stronger database audit than the implementation performs.

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
- [Technical companion](../README.md)
