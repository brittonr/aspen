# Aspen Dataspace Access Cache

## ADDED Requirements

### Requirement: Canonical key projection

r[aspen.dataspace_access_cache.projection] The cache MUST key every entry by the BLAKE3 digest of a canonical projection over dataspace identity, normalized access arguments, and optional capability context. The projection MUST exclude ambient state that does not change the access result.

#### Scenario: Same access projects to the same key

r[aspen.dataspace_access_cache.projection.equal]
- GIVEN two accesses with equal dataspace identity, arguments, and context
- WHEN the cache projects both keys
- THEN the keys MUST be equal

#### Scenario: Distinct accesses project to distinct keys

r[aspen.dataspace_access_cache.projection.distinct]
- GIVEN two accesses whose dataspace identity or arguments differ
- WHEN the cache projects both keys
- THEN the keys MUST differ

### Requirement: Explicit capacity

r[aspen.dataspace_access_cache.bound] The cache MUST require an explicit capacity. The cache MUST NOT exceed that capacity and MUST NOT assume an implicit default bound.

#### Scenario: Operator configures a capacity

r[aspen.dataspace_access_cache.bound.configured]
- GIVEN an operator supplies an explicit capacity
- WHEN the cache is built
- THEN the cache MUST retain at most that many entries

#### Scenario: Insertion at capacity

r[aspen.dataspace_access_cache.bound.full]
- GIVEN the cache is at capacity
- WHEN an insertion requires a slot
- THEN the cache MUST evict by policy
- AND the entry count MUST stay at or below capacity

### Requirement: Deterministic retention decisions

r[aspen.dataspace_access_cache.decision] The decision core MUST decide eviction and promotion from supplied values only. It MUST read no clock, memory, or scheduler state. The promotion decision MUST honor an explicit threshold where 0 means strict LRU and 100 means FIFO.

#### Scenario: Strict LRU retention

r[aspen.dataspace_access_cache.decision.strict_lru]
- GIVEN a threshold of 0 and an entry now qualifies as most recently used
- WHEN the core evaluates promotion
- THEN the entry MUST promote

#### Scenario: FIFO retention

r[aspen.dataspace_access_cache.decision.fifo]
- GIVEN a threshold of 100 and any entry age
- WHEN the core evaluates promotion
- THEN the entry MUST NOT promote

### Requirement: Deferred release ordering

r[aspen.dataspace_access_cache.deferral] The core MUST return release plans so the shell drops results after it releases the guard. The shell MUST NOT run user destructors inside the synchronization boundary.

#### Scenario: Result evicted under contention

r[aspen.dataspace_access_cache.deferral.evicted]
- GIVEN a result is evicted while other threads wait
- WHEN the shell processes the release plan
- THEN the guard MUST be released first
- AND the result MUST drop only after release

### Requirement: Adapter authority

r[aspen.dataspace_access_cache.boundary] A cache hit MUST NOT change dataspace semantics. The cache MUST NOT claim that a hit proves record freshness or vat authority. The cross-repo retention contract MUST be recorded in repo docs.

#### Scenario: Over-claim of cache authority

r[aspen.dataspace_access_cache.boundary.overclaim]
- GIVEN a consumer claims that a cache hit proves dataspace freshness
- WHEN the claim boundary is verified
- THEN the claim MUST fail unless the dataspace adapter owns it

### Requirement: Fixture coverage

r[aspen.dataspace_access_cache.verification] Positive, negative, and boundary fixtures MUST cover projection stability, bounded retention, retention decisions, deferred release, and the claim boundary.

#### Scenario: Complete focused matrix passes

r[aspen.dataspace_access_cache.verification.matrix]
- GIVEN decisions, fixtures, documentation, and dependency evidence are complete
- WHEN focused workspace, Clippy, Cairn, and Nix verification runs
- THEN in-bounds inputs MUST behave as declared
- AND every missing-bound, reversed-watermark, boundary, or over-claim input MUST fail as declared
