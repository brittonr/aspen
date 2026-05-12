## Context

Previous Raft drains removed direct log, snapshot, TTL, batcher, and apply facts. The remaining operational Raft markers are localized to apply-request and batcher accounting/contiguity. Chain hash/verify markers are a different class and should be handled by explicit crypto-boundary classification.

## Goals / Non-Goals

**Goals:**
- Remove non-crypto Raft arithmetic/accounting `external_body` markers.
- Keep proofs small and rooted in existing specs.
- Preserve runtime behavior.

**Non-Goals:**
- Proving Blake3/collision resistance or chain cryptographic assumptions.
- Rewriting the write-batcher implementation outside proof/spec alignment needs.

## Decisions

### 1. Separate operational proofs from chain trust

**Choice:** This OpenSpec covers only apply-request and write-batcher markers.

**Rationale:** Operational arithmetic should be dischargeable; crypto chain assumptions need separate classification.

### 2. Use saturated-span precedent for contiguity

**Choice:** Reuse the prior saturated inclusive-span pattern from chain length and batcher flush if contiguity touches `u64::MAX` spans.

**Rationale:** Prior proof closures exposed exact-overflow boundary behavior that must be modeled explicitly.

## Risks / Trade-offs

**Arithmetic edge cases** → Add branch-specific assertions and tests at `u64::MAX`/large byte-count boundaries when semantics are clarified.

**Overbroad Raft root churn** → Commit apply and batcher proof slices separately if they require different helper lemmas.
