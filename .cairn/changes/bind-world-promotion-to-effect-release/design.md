## Context

Molten effect logs distinguish replay from live dispatch. Existing delivery paths also use idempotency and durable state. World branching adds a new boundary: losing or simulated branches can contain valid-looking intents that must not escape.

The branch-head store and effect outbox can share one local transaction. External systems cannot join that transaction in the general case.

## Decisions

### Decision: Separate immutable intents from mutable release state

**Choice:** A candidate world commit references canonical effect intent state. Promotion creates a detached `world-promotion-record-v1` and durable release reservations.

The candidate commit does not include its own promotion record or reservation identities.

**Rationale:** This avoids circular commit identity. It also preserves the same candidate identity before and after policy review.

### Decision: Atomically publish active head and reservations

**Choice:** The local promotion transaction rechecks expected active head, candidate identity, branch policy, authority, intent closure, and reservation set.

It then updates the active head and inserts every release reservation as one atomic store operation. Any local failure leaves both unchanged.

**Rationale:** An active head without reservations can lose effects. Reservations without the active head can release a losing branch.

### Decision: Treat eligibility as atomic, not external completion

**Choice:** Transaction success makes intents eligible for dispatch. A separate dispatcher claims committed reservations and reruns current effect admission.

No receipt claims atomic completion in an external system.

**Rationale:** External effects cannot participate in a universal local transaction.

### Decision: Use stable operation and release identities

**Choice:** Each promotion has an explicit operation identity. Each release reservation uses domain-separated BLAKE3 framing over promotion, candidate, and exact semantic intent identities.

Retries reuse these identities. They never mint new logical operations because an outcome is unknown.

**Rationale:** Stable identities support deduplication, reconciliation, and first-divergence diagnosis.

### Decision: Model the complete effect lifecycle

**Choice:** Store intent, reservation, claim, attempt, observation, acknowledgment, uncertain, conflict, denied, reconciled, and abandoned facts as distinct typed records.

A state transition requires the exact prior state and current generation.

**Rationale:** One `pending` flag hides crash windows and encourages false exactly-once claims.

### Decision: Reconcile uncertain local publication before retry

**Choice:** Use Transactional Reconciliation Core to classify local commit outcomes. An unknown result triggers operation-identity observation before any repeat mutation.

Conflicting observations block dispatch and require explicit recovery.

**Rationale:** Blind retry after an unknown commit can duplicate head movement or reservations.

### Decision: Recheck authority immediately before dispatch

**Choice:** Promotion admission and dispatch admission are separate. The dispatcher rechecks current capability, Basalt policy, semantic handler, adapter generation, and reservation ownership.

A later denial records a blocked reservation. It does not rewrite the promoted commit.

**Rationale:** Authority can change after promotion and before external execution.

### Decision: Record observations through successors or sidecars

**Choice:** Effect attempts and observations never mutate the promoted commit. A later world commit can reference updated effect state, while detached evidence can report bounded observations.

**Rationale:** Immutable world identity must remain stable as external outcomes arrive.

## Rollout

1. Add pure promotion and reservation planning without dispatch.
2. Add local atomic head-and-reservation transactions with deterministic fake effects.
3. Add crash recovery and uncertain-commit reconciliation.
4. Pilot one idempotent local effect adapter.
5. Add remote effects only after duplicate and unknown-outcome rails pass.

## Risks / Trade-offs

- Durable reservations can remain blocked after authority changes. Operators need explicit inspect, deny, replace, or abandon actions.
- Some external APIs lack idempotency support. Such adapters require conservative at-least-once or manual reconciliation profiles.
- The local store can report uncertain commit outcomes. Observation-first recovery prevents blind retries.
- Follow-up commits increase history volume. They preserve causal truth instead of mutating prior worlds.
