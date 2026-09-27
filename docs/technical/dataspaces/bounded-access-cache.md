# Bounded Access Cache

The dataspace-access cache is an advisory optimization, not an authority store or a freshness protocol. This article examines its deterministic key and retention decisions, mutex boundary, and concurrent-miss behavior. Read the governing [bounded cache contract](../../dataspace-access-cache.md) first. The [architecture](../../architecture.md) supplies the broader pure-core/admitted-shell boundary. Return to the [Technical companion](../README.md).

## Projection is an explicit semantic choice

The pure core accepts `AccessProjection` containing `dataspace_identity`, `normalized_arguments`, and optional `capability_context`. `project_key` validates bounded text and argument counts, then uses BLAKE3's derive-key mode with a fixed domain context. Labeled, length-delimited fields distinguish the dataspace, argument count, individual arguments, and capability text. It does not inspect clocks, scheduler state, process memory, or adapter handles. See the [core cache implementation](../../../crates/molten-core/src/fabric_durability/cache/mod.rs).

The name `normalized_arguments` describes an input obligation, not normalization performed by this function. Arguments are hashed in supplied order. Two callers that encode the same semantic access differently can therefore create different keys. Conversely, omitting relevant context can make semantically different accesses share a key. A cache cannot repair an insufficient projection by observing that a lookup succeeded.

There is a specific representation caveat in the inspected code: absent capability context is hashed using `unwrap_or("none")`. Therefore `None` and `Some("none")` have identical encoded capability text when other fields agree. The governing document describes an optional projection but does not specify this alias. This article does not claim injective encoding of the optional field or invent a resolution; review that implementation detail before using absence versus literal `"none"` as a security-relevant distinction. The cache remains non-authoritative under the [governing claim boundary](../../dataspace-access-cache.md).

## Capacity, watermarks, and promotion

`Policy::new` rejects zero capacity, capacity above its implementation bound, invalid watermarks, and promotion thresholds above 100. Valid watermarks satisfy `0 <= low < high <= capacity`, with positive high watermark. There is no implicit capacity selected by the constructor.

An insertion plan checks active count against capacity, requires an eviction-order entry for every active entry, and rejects duplicate eviction keys. If the current active count reaches the high watermark, it retains the low-watermark count before inserting the new entry. Thus hysteresis can evict several values at once, and the post-insertion count is low watermark plus one—not necessarily the low watermark itself.

Promotion uses an access counter since the previous promotion. Threshold zero promotes every hit; 100 never promotes hits and gives FIFO behavior; intermediate thresholds compare the counter against the declared integer. The threshold is not a sampled probability or a percentage of accesses. The shell increments counters with saturating arithmetic and resets a promoted counter.

## Worked reasoning: why a hit changes eviction

Take an illustrative policy with capacity 2, high watermark 2, and low watermark 1. Load `alpha`, then `beta`. The eviction order is oldest first: `alpha`, `beta`.

With threshold zero, a hit on `alpha` moves it to the back. Loading `gamma` at the high watermark retains one old entry, evicts `beta`, and inserts `gamma`. The resulting keys are `alpha` and `gamma`.

With threshold 100, the same hit does not move `alpha`. Loading `gamma` instead evicts `alpha`, leaving `beta` and `gamma`. Both executions satisfy the same capacity constraint; they differ only in retention policy. The [shell tests](../../../src/runtime/dataspace/cache/tests.rs) contain this LRU/FIFO contrast.

## Synchronization and object lifetime

The shell's `Store<Value>` owns a mutex-protected map and eviction deque. `lookup_or_load` projects the key, checks for a hit, and invokes the supplied loader only after the lookup guard is gone. Loader failures remain `LookupError::Load` with the original error value. On success it acquires the mutex again and either inserts the candidate or reuses an entry another caller already inserted. This is visible in the [shell implementation](../../../src/runtime/dataspace/cache/mod.rs).

That second check is not single-flight loading. Two simultaneous misses can both run loaders, with only one value retained for the key. The losing candidate is placed in a deferred-release collection. Evicted cached values are likewise moved into that collection on the successful mutation path, and the collection is dropped after leaving the guard scope. Hits clone `Arc`, not user-defined value clone behavior.

The destructor boundary and the capacity boundary are distinct. Eviction releases the store's ownership, but callers may retain `Arc` values after eviction. A bound on active entries is not a byte budget for arbitrary values, a bound on all outstanding references, or a bound on concurrent loader allocations. Failure paths and panic/unwind behavior also merit separate review rather than extrapolation from the successful-path deferral test.

## Verification and limits

Suggested review targets include malformed projection before loading, loader-error preservation, LRU/FIFO divergence, watermark batch eviction, racing misses, and destructors that inspect the store. Existing tests include an eviction destructor probe using `try_len` and a malformed projection that leaves the loader uncalled. They were inspected, not executed for this article.

A hit establishes only local presence under the projected key. It does not establish current policy, capability validity, remote availability, durable storage, or production readiness. No performance parity with a reference project is claimed. The core makes in-memory decisions without synchronization or I/O; the shell owns both storage and locking.

## Sources

- [Bounded dataspace-access cache contract](../../dataspace-access-cache.md)
- [Architecture and authority separation](../../architecture.md)
- [Pure projection and retention laws](../../../crates/molten-core/src/fabric_durability/cache/mod.rs)
- [Mutex-owning runtime shell](../../../src/runtime/dataspace/cache/mod.rs)
- [Cache behavior and destructor tests](../../../src/runtime/dataspace/cache/tests.rs)
