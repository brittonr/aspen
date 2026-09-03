# Prolly semantic-state map

Molten uses `prolly-semantic-map-v1` for keyed semantic-state roots. The map is product-local. It is not a universal world representation.

## Ownership

The pure core owns:

- profile and node validation;
- canonical BLAKE3 identities;
- deterministic split decisions;
- map build, read, edit, and diff plans;
- closure, reachability, and GC plans;
- bounded benchmark and differential facts.

The shell owns:

- immutable block reads and staged writes;
- Redb transactions;
- compare-and-advance publication;
- unknown-outcome reconciliation;
- restart and storage observations;
- retention rechecks and block deletion.

The map does not own branch authority, semantic merge policy, effects, replication policy, snapshot policy, or release decisions.

## Profile identity

`config/prolly-map/profile.ncl` owns the reviewable profile. Rust revalidates the generated projection.

The profile identity is:

`blake3:820d0424eac0ce727d80750e485dcaac320137baa5b4ef21d0d67708ca8a41d5`

The profile binds:

- canonical byte key and value codecs;
- unsigned lexicographic key order;
- `molten-prolly-node-binary-v1`;
- content-identity-core tagged framing;
- five BLAKE3 domains;
- the boundary seed;
- exact encoded-byte accounting;
- node byte bounds;
- fanout bounds;
- key, value, entry, height, diff, and graph limits;
- format version 1.

Changing any structural field changes or invalidates the profile identity.

## Node format

Every node starts with the `MPL1` magic, format version, node kind, profile reference, and bounded item count.

A leaf stores sorted unique key and value byte pairs. An internal node stores ordered, non-overlapping child ranges, child identities, and encoded lengths.

Node identity uses a separate leaf or internal BLAKE3 domain. Root identity binds the profile, top node, height, and entry count.

Unknown versions, trailing bytes, malformed lengths, duplicate keys, overlapping ranges, wrong profiles, oversized nodes, missing blocks, cycles, and identity mismatches fail closed.

## Boundaries

Split decisions use:

- the profile boundary domain and seed;
- canonical key bytes;
- current exact encoded size.

Values affect encoded size but do not provide boundary entropy. The split probability increases toward the target and maximum sizes. The maximum size always forces a split.

The standard profile uses these bounds:

| Field | Value |
| --- | ---: |
| Minimum node bytes | 256 |
| Target node bytes | 1,024 |
| Maximum node bytes | 4,096 |
| Minimum fanout | 2 |
| Target fanout | 4 |
| Maximum fanout | 8 |
| Maximum key bytes | 64 |
| Maximum value bytes | 1,024 |
| Maximum entries | 4,096 |
| Maximum tree height | 16 |

## History independence

The pilot uses a rebuild-first edit mechanism. It applies admitted edits to a complete supplied snapshot, sorts the resulting map, and rebuilds the canonical tree.

This mechanism is weaker than a path-local update algorithm, but it is easier to audit. Equal canonical maps always rebuild to the same root.

The edit plan stages only blocks that are not in the prior closure. Unchanged blocks keep their identities and remain shared.

Compaction uses the same canonical build. A compaction that changes the root without changing entries is a determinism defect.

## Reads and diffs

Point and range reads operate on fully validated supplied blocks.

Diff reports complete added, removed, and modified entries. It counts equal node identities that it can skip under the declared BLAKE3 collision-resistance assumption.

Diff never selects a merge winner or moves a branch head.

## Storage and recovery

`ProllyBlockStorePort` is application-owned. `LocalProllyBlockStore` implements it with one Redb database in the node storage namespace.

Blocks are immutable. Root publication uses a Redb compare-and-advance transaction.

If publication may have completed without an acknowledgement, the service reads the durable root once. It reports applied, not applied, or unknown. It does not retry the mutation blindly.

Canonical publication receipts do not grant future mutation or deletion authority.

## Reachability and GC

The core receives all roots, pins, node identities, and graph facts. It returns reachable and candidate-unreachable identities.

A missing or incomplete graph fact makes the plan incomplete. The plan never grants deletion authority.

The shell deletes candidates only after exact root and pin comparison, current-generation evidence, retention policy approval, and deletion authority.

## DoltLite differential boundary

The Prolly map compares ordered text rows and outcomes with the pinned DoltLite oracle.

DoltLite roots and Molten roots use different formats and domains. Differential comparison never requires root equality. Agreement is evidence for one case, not proof of correctness.

## Proof obligations

The pilot records obligations for sorted uniqueness, search containment, boundary determinism, edit preservation, diff soundness, and reachability.

The reference Trellis revision is `0bf65150d4c75da5887d5cc53392c3da6b94b9d2`.

Bounded model tests cover these obligations. Formal refinement to the production node codec remains open. No evidence claims collision impossibility or database correctness.

## Benchmark and extraction gate

`config/prolly-map/benchmark.ncl` defines a logical, bounded, rebuild-first cohort. It records sharing, diff records, bytes, restart, GC candidates, and adversarial maximum-node behavior.

Raw timings and counts are measurements. They are not correctness evidence.

The existing world benchmark classifier sees one credible consumer. Its result is `retain-current`. It does not create a repository or approve a dependency.

Extraction needs two credible consumers with the same map semantics, authority boundary, and evidence contract. Choregraph history does not meet that condition by similarity alone.

## Migration

Format version 1 has no implicit migration. A profile or format change creates a different identity domain.

Schema Migration Core can plan an explicit migration. The shell must read the old map, build the new map under the target profile, compare normalized entries, and publish through a new compare-and-advance operation.

Old blocks remain subject to their original retention and pin policy.

## Non-claims

The pilot does not prove:

- BLAKE3 collision impossibility;
- database correctness;
- branch authority or merge correctness;
- effect safety;
- replication correctness;
- GC execution safety;
- universal performance;
- production readiness;
- release eligibility.
