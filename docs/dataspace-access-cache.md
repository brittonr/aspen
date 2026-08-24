# Bounded dataspace-access cache

Molten provides an advisory cache for repeated dataspace access results.
The cache does not change dataspace semantics or authority.

## Functional core

`molten-core` owns deterministic key and retention decisions.
The key is a domain-separated BLAKE3 digest over these fields:

- the dataspace identity
- normalized access arguments
- an optional capability-context projection

The projection excludes clocks, memory state, scheduler state, and other ambient values.

Every retention policy supplies an explicit capacity.
It also supplies high and low watermarks plus a promotion threshold.
There is no default capacity.

A threshold of `0` gives strict LRU behavior.
A threshold of `100` gives FIFO behavior.
Intermediate values promote an entry after the declared access count.

The core returns insertion, eviction, promotion, and deferred-release plans.
It performs no I/O and owns no synchronization.

## Imperative shell

The runtime shell owns the bounded store and its mutex.
A cache miss calls the supplied dataspace loader outside the mutex.
The shell returns the loader error without changing its meaning.

The shell moves evicted values into a deferred-release list.
It releases the mutex before it drops that list.
As a result, a user destructor does not run inside the synchronization boundary.

The store uses `Arc` values for cache hits.
An `Arc` clone does not invoke user-owned clone code while the mutex is held.

## Cross-repository contract

The local policy follows the planned product-neutral `bounded-cache` retention contract.
No published `bounded-cache` component is available in the local OnixResearch workspace.
Molten therefore owns this bounded implementation until a compatible immutable component is published.

A future adoption must preserve these facts:

- explicit capacity and watermarks
- deterministic key and retention decisions
- deferred release after the guard
- unchanged dataspace and authority semantics
- positive and negative compatibility evidence

A sibling checkout is not a product dependency.

## Claim boundary

A cache hit proves only that the local store contains a value for the projected key.
It does not prove record freshness, vat authority, policy currentness, remote availability, durability, or release readiness.

The `adanil-code/LRUHashTable` project is a bounded design reference for lazy promotion and deferred release.
Molten does not claim API, implementation, or performance parity with that project.
