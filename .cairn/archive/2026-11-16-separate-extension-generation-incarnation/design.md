## Context

Internal system-extension transitions previously represented both generation and incarnation as raw `u64`. Canonical Preserves records and public numeric reporting interfaces already encode them as unsigned integers; unrelated promotion, fabric durability, and other generation domains remain independent.

## Decisions

### Decision: Distinct local full-width domains at internal interfaces

**Choice:** Define two non-interconvertible, copyable newtypes with explicit `new(u64)`, `get()`, and checked increments. Propagate them through lifecycle, dispatch, manifest, facade and native-host model fields. Convert at the existing `u64` serialization/API edges, including journal and wire decoders.

**Rationale:** The full `u64` range, including values above `2^63-1`, remains accepted, unlike upstream's u63-oriented generation representation. Keeping Preserves record names, field order, numeric representation and BLAKE3 hashing unchanged preserves all canonical bytes and content refs. No aliases or cross-domain `From` conversions hide mistakes.

## Risks / Trade-offs

- Every transition-facing caller must migrate explicitly; compiler errors reveal missed boundaries. Compile-fail tests prove swapped domains cannot enter both event construction and transition-facing fields. Runtime tests still prove stale, restart, and both overflow paths. Golden references guard the byte identity contract.
