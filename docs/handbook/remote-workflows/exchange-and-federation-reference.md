# Exchange and federation reference

Mode: Reference

Use this page to classify a result before deciding what to do with it. Artifact exchange, remote dataspace delivery, federation inventory pulls, and bounded DAG synchronization expose different inputs and evidence. They are not interchangeable implementations of “sync everything.” Return to the [Handbook](../README.md); the [DAG closure companion](../../technical/replication/bounded-dag-closure-exchange.md) supplies the conceptual background.

**Status:** implementation and fixture sources were inspected; no commands or runtime scenarios were executed for this batch. Names below are Rust APIs and record fields, not shell subcommands. Canonical Preserves bytes and BLAKE3 define canonical artifact identity, not these Rust layouts.

## Exchange inputs and outputs

| Item | Fields or values to retain | Owner and operational meaning |
|---|---|---|
| `Repro` | `ticket`, `bundle_ref`, `receipt_value` | Iroh exchange helper returns identity and exchange evidence. |
| `FetchBundleInput` | `root`, `ticket`, `expected_bundle_ref`, `peer`, `out`, `ledger_root` | Caller selects the storage capability, expected identity, and optional output effects. |
| `ChainSegment` | `ticket`, `bundle_ref`, `chain`, `anchor_ref`, `head_ref`, `receipt_value` | Chain exchange identifies a scoped segment rather than a generic reproduction bundle. |
| `PublishChainSegmentInput` | `iroh_root`, `ledger_root`, `chain`, `anchor_ref`, `expected_head`, `node`, `fork_policy` | Publisher supplies chain scope and explicit fork policy. |
| `FetchChainSegmentInput` | `iroh_root`, `ticket`, `expected_bundle_ref`, `peer`, `ledger_root`, `fork_policy` | Receiver supplies destination and its own fork policy. |

The [exchange definitions](../../../src/iroh/parts/exchange/p000/body.rs) also bound chain bundles at 100,000 artifacts and 10,000 checkpoints. These are separate dimensions, not a byte budget. The reproduction-bundle helper shown there uses `iroh-local:` tickets and local capability-rooted reads and writes. An `out` destination receives textual Preserves; the stored blob uses canonical bytes. Do not compare a text-file hash with a canonical bundle reference without applying the canonical encoding rules.

## Dataspace evidence and ownership

| Item | Relevant fields | Responsible boundary |
|---|---|---|
| `Envelope` | sender peer/actor, target peer, topic, operation, payload, content/capability/evidence refs, sequence, operation ref | Envelope construction and parsing establish canonical structure and identity. |
| `Delivery` | envelope, transport receipt | Transport checks delivery context and content availability. |
| `DeliveryEvidence` | peer-bootstrap, capability, policy, resource, authority refs | Caller supplies admission evidence; helper validates required reference lists and inclusion. |
| `RemoteSession` | reference, owner, receiver, topic, generation, state | Registry records receiving lifetime identity. |
| `SessionApplied` | owner, session ref, events, admission receipt, optional applied assertion, turn context | Session-aware application records the owner used for runtime changes. |
| `ClosedRemoteSession` | retraction events, cleanup, receipt, before/after state refs | Lifecycle helper describes local cleanup, not remote erasure. |

The operation enum accepts `message`, `assert`, `retract`, and `observe`. Transport labels distinguish `iroh-local-gossip` from `iroh-gossip`. The general replay-event bound is 4,096; the session module separately declares 4,096 session retractions. Do not infer an exactly-once delivery guarantee from an operation reference or replay receipt.

## Federation resource and policy fields

| Structure | Fields | Interpretation |
|---|---|---|
| `Resource` | `resource_type`, `resource_ref`, `schema`, `transport`, `source_peer` | Announcement metadata; nonempty strings are not proof of adapter support. |
| `Inventory` | peer, resources, delegates, signer, trust root, canonical value/ref | Describes supplied resource inventory and its signature context. |
| `Delegate` | resource ref, capability, signer, trust root, canonical value/ref | Binds a resource-specific delegation claim checked against the receiver's expected context. |
| `PullPolicy` | allowed types, optional required delegate capability, delegate trust root/key, resource/import limits | Receiver-selected filters and bounds. |
| `Pull` | peer, imported refs, skipped refs, denied refs, receipt | Result partitions resources; failure may coexist with completed imports. |

[Resource constants](../../../src/federation/parts/mod/p000/body.rs) include artifact, chunk-manifest, chunk, doc-metadata, catalog-metadata, receipt, provenance, transcript, protocol, and schema. The struct validator is not a closed enumeration validator. Ledger pulls additionally compare the artifact's actual kind and canonical hash with the advertised resource.

An empty allowed-type list means no type restriction in this helper, not deny-all. `PullPolicy::allow_all()` also leaves delegation optional and uses unbounded policy counters; the implementation still has separate inventory limits. These API defaults are not deployment recommendations.

## Worked result interpretation

Suppose a policy-admitted inventory contains two locally readable resources and one disallowed type. The [pull loop](../../../src/federation/parts/mod/p002/body.rs) can import the first two, deny the third, and return a receipt whose overall decision is `fail`. This means partial effects occurred; it does not mean rollback. An already present resource is considered for skipping only after type and required-delegate checks.

If source reading or identity/kind verification fails for a resource, it is denied. A destination import error can instead propagate from the helper. Preserve destination evidence and distinguish a returned mixed result from an error with uncertain earlier effects before planning another attempt.

## Limits on authority and closure

The inspected [federation signature helper](../../../src/federation/parts/mod/p003/body.rs) hashes canonical payload bytes with signer, purpose, trust-root, and key material. Do not describe it as public-key verification, remote membership, or proof of a production trust service.

Locator evidence is hint-only until fetched identity, verification, local admission, authority, policy, and resource evidence are supplied. Even its admission helper checks reference presence and shape, not every referenced artifact's substantive truth. General DAG completion remains relative to receiver roots, strategy, epoch, generation, and policy under the governing contract; an inventory pull is not that closure proof.

## Sources

- [Handbook](../README.md)
- [Governing architecture](../../architecture.md)
- [DAG synchronization contract](../../dag-sync.md)
- [Bounded DAG closure companion](../../technical/replication/bounded-dag-closure-exchange.md)
- [Exchange types and reproduction-bundle path](../../../src/iroh/parts/exchange/p000/body.rs)
- [Dataspace envelope and evidence types](../../../src/remote/parts/dataspace/p000/body.rs)
- [Federation structures and policy defaults](../../../src/federation/parts/mod/p000/body.rs)
- [Federation pull decisions](../../../src/federation/parts/mod/p002/body.rs)
- [Locator admission evidence](../../../src/federation/parts/mod/p004/body.rs)
