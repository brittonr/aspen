# History-Independent Semantic Maps

Molten's Prolly map gives a bounded keyed semantic state a canonical tree representation. Its important property is not that every edit is cheap, but that equal canonical maps under one exact profile rebuild to the same root. This article explains that property and its limits. Read the [Prolly semantic-state map contract](../../prolly-semantic-state-map.md) first; the [Technical companion](../README.md) links related world-state topics.

## Identity includes the representation contract

A map root cannot be understood independently of its profile. The profile binds key and value codecs, unsigned lexicographic key order, binary node format, tagged identity framing, domain separation, boundary seed, exact byte accounting, and resource limits. Changing structural profile fields changes or invalidates the profile identity. Consequently, equal application-level values encoded under different profiles are not promised equal map roots.

Nodes use the `MPL1` format described in the [governing node contract](../../prolly-semantic-state-map.md#node-format). Leaves contain sorted unique byte-key/value pairs. Internal nodes contain ordered, non-overlapping child ranges, identities, and encoded lengths. The root additionally binds profile identity, top-node identity, height, and entry count. These are protocol bytes and explicit fields, not a hash of Rust object layout.

The standard profile's 256-byte minimum, 1,024-byte target, and 4,096-byte maximum node sizes are bounded-cohort parameters, not universal recommendations. Fanout, entry count, key size, value size, height, graph facts, and diff records impose further independent limits.

## Deterministic boundaries are content-dependent, not random effects

[boundary_decision](../../../crates/molten-core/src/prolly_map/tree/build.rs) returns a forced split at the maximum encoded size and no split below the minimum. Between those limits it hashes the profile seed, canonical key bytes, and encoded size, derives a bounded score, and compares that score with a size-dependent threshold. No random-number generator or clock participates.

Values influence exact encoded size but are not independent boundary entropy. Therefore changing a value's length can change boundaries even when all keys remain fixed. Leaf construction checks whether appending an entry would exceed the maximum and starts another chunk when needed. Internal construction also observes fanout limits and deterministically rebalances a small final group.

History independence is thus a property of the whole canonical construction, including grouping and root metadata. It is not established merely by sorting the leaves or hashing each value independently.

## Rebuild-first editing and sharing

[plan_edits](../../../crates/molten-core/src/prolly_map/operations.rs) validates the complete supplied snapshot, copies its entries into an ordered map, applies the admitted edits in order, and invokes the canonical builder on the resulting entries. Insert-on-existing, update-on-missing, and delete-on-missing are errors; edit sequences are not silently normalized into arbitrary upserts.

The plan compares prior closure identities with new block identities. Only blocks absent from the prior closure are staged, while equal blocks remain shared. This is immutable block reuse after rebuilding, not a path-local update algorithm. It is important to separate computational work from persistent writes: avoiding a block rewrite does not imply avoiding traversal, validation, allocation, or reconstruction.

## Worked reasoning: different histories, one final map

As an illustrative example, start with byte key `alpha` mapped to byte value `1`. History A inserts `beta=2`, updates `alpha=3`, and deletes `beta`. History B simply updates `alpha=3`. Both admitted histories finish with the identical canonical map containing only `alpha=3`.

Under the same validated profile, rebuilding those final entries yields the same tree and root. The property does not say the plans have identical edit counts or staged-block sets relative to every possible predecessor. It says insertion history is absent from the final canonical map identity.

Now change History A's last action to an update of missing key `gamma`. That sequence is denied rather than equated with History B. Alternatively, encode `3` differently under another profile: application-level similarity no longer establishes equal canonical input. These boundaries prevent “history independent” from becoming a claim that every operation commutes or every representation is interchangeable.

## Diff evidence and publication

The [diff implementation](../../../crates/molten-core/src/prolly_map/operations.rs) validates both complete snapshots, compares sorted entries, and reports added, removed, and modified records. It records `skipped_equal_nodes` from equal closure identities; even the equal-root case validates snapshots first. The governing contract describes identities it can skip, but this counter should not be read as measured avoided I/O or proof of a lazy subtree traversal. The observed implementation is narrower than that performance interpretation.

[Publication](../../../src/prolly_map/service.rs) stages immutable blocks and then performs compare-and-advance on the map root and generation. An uncertain outcome triggers one durable-root read: exact successor means applied, exact predecessor means not applied, and another state remains unknown. Publication receipts explicitly deny future mutation and deletion authority.

## Review, verification, and limits

Suggested verification compares final normalized entries and roots across admitted edit histories, then changes only a profile field to ensure the representation boundary remains explicit. Review exact byte accounting around forced splits and internal rebalancing. For storage, lose a publication acknowledgement and inspect reconciliation without another mutation attempt. These checks are suggested, not executed here.

History independence is an in-memory deterministic law under the chosen hash assumptions, not a proof of collision impossibility, database correctness, or production readiness. Diff selects no merge winner. Reachability produces deletion candidates, not deletion authority; execution still requires current roots and pins, generation evidence, retention approval, and authority. Compaction preserving entries should preserve the root, while profile migration requires explicit reconstruction and publication in the new identity domain.

## Sources

- [Prolly semantic-state map contract](../../prolly-semantic-state-map.md)
- [World-state diff and merge contract](../../world-state-diff-and-merge.md)
- [Canonical tree construction and boundaries](../../../crates/molten-core/src/prolly_map/tree/build.rs)
- [Edit and diff plans](../../../crates/molten-core/src/prolly_map/operations.rs)
- [Publication, reconciliation, and GC admission](../../../src/prolly_map/service.rs)
- [Technical companion](../README.md)
