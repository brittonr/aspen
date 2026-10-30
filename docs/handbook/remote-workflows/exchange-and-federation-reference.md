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

## Federation admission and receiver planning

The production surface is the optional manifest-installed federation service,
not the former local-root copying helper. The old include-based implementation
is confined to `cfg(test)` simulation fixtures and cannot satisfy live admission.

| Record or API | Meaning | Authority boundary |
|---|---|---|
| `ManifestInput` / `admit_manifest` | Exact admitted extension, generation, static peers, DAG binding, finite bounds and reviewed profile refs | Required bindings must resolve through normal system-tier/fabric admission; referenced policies are bound by the admitted extension. |
| `SignedRecord` | Canonical announcement, inventory, delegate, request or response domain, public signer, key generation and freshness | Uses opaque production Ed25519 adapters; a verified signature is not local import authority. |
| `PeerHint` / `observe_locator` | Static, endpoint, gossip, tracker, DHT, pkarr, probe or catalog candidate observation | All locators remain hint-only. Optional hints cannot substitute for configured peer admission. |
| `PullPlan` | Receiver missing refs, explicit conflicts, byte/node/chunk budgets, deadline and generation | Divergent names do not choose winners and are excluded from automatic import. |
| `Service` / `execute_pull` | Supervised receiver session and host-owned typed DAG effect | The existing DAG shell still obtains current authority/resources and verifies content. |
| `PolicyPort` / `import::commit_verified` | Local schema, policy, capability, provenance, resource, retention and import gates | All gates bind the exact local session, generation and head before trusted ledger mutation. |

## Transfer, quarantine and commit order

`NetworkDagPort` sends an admitted signed request through actual cross-process
Iroh transport and accepts only a signed response correlated to that request and
object. `quarantine::VerificationPort` verifies canonical identity and invokes the
admitted content adapter in a capability-rooted quarantine store. Quarantine is
not the trusted artifact registry and grants no installation, execution or merge
authority.

After complete correlated DAG verification, the receiver parses canonical
artifact bytes, invokes every local admission gate, previews the commit
transition, persists intent, and imports through the capability-rooted ledger.
A post-effect persistence error is uncertainty requiring readback, not a policy
denial or a reason for blind retry. A partial DAG run does not start registry
import; domain conflicts never acquire federation merge authority.

## Bounds and evidence interpretation

Inventory entries/bytes, query intervals, concurrent sessions, cumulative
network bytes, DAG nodes, chunks, retries, session ticks, freshness and retained
status are explicit bounds. Local status contains refs and finite diagnostics,
not raw payloads, key material or credentials. Successful completion is scoped
to one receiver plan and observation window; it does not prove permanent
convergence or remote correctness.

The DAG package's historical live-loopback conformance adapter transmits
request metadata and uses fixture admission refs. It is not a production content
fetch adapter. Neither prerequisite archive status, filesystem copying nor
signature verification alone establishes live signed pull/import evidence.

The source descriptions above are not an execution report. Live-loopback,
distinct-process, offline-verifier and adversarial simulation receipts must be
produced by the corresponding runnable fixtures before making those claims.

## Sources

- [Handbook](../README.md)
- [Governing architecture](../../architecture.md)
- [DAG synchronization contract](../../dag-sync.md)
- [Bounded DAG closure companion](../../technical/replication/bounded-dag-closure-exchange.md)
- [Exchange types and reproduction-bundle path](../../../src/iroh/parts/exchange/p000/body.rs)
- [Dataspace envelope and evidence types](../../../src/remote/parts/dataspace/p000/body.rs)
- [Receiver decision core](../../../crates/molten-core/src/federation/planner.rs)
- [Production signed network records](../../../src/federation/envelope/network.rs)
- [Supervised pull execution](../../../src/federation/transfer/execution.rs)
- [Local import policy](../../../src/federation/admission/rules.rs)
- [Owner-defined DAG binding](../../../src/dag_sync/binding.rs)
- [Historical simulation-only federation fixtures](../../../src/federation/parts/mod/p000/body.rs)
