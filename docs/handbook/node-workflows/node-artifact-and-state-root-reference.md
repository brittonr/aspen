# Node artifact and state-root reference

Mode: Reference

Use this reference when inventorying an isolated node or assembling an evidence bundle. Paths below are relative to the explicitly selected state root unless marked as exports. They identify implementation locations, not a stable external backup format. The [node-state authority contract](../../node-state-filesystem-authority.md) governs filesystem access; the [capability-rooted state companion](../../technical/storage/capability-rooted-node-state.md) explains retained authority. This inventory is source-checked, not runtime-verified for this document.

## Root lifecycle artifacts

The names are declared by the [daemon constants](../../../src/node/parts/daemon/p000/body.rs). Publication belongs to the daemon shell, while canonical values come from the profile, identity, and runtime modules.

| Relative path | Publishing owner | Operational meaning and limit |
| --- | --- | --- |
| `config.preserves` | Daemon initialization and profile resolution | Selected node configuration; not independent proof that every configured adapter is live. |
| `profile-resolution.preserves` | Daemon initialization | Records profile resolution, including the no-profile local-fixture caveat. Preserve it with configuration. |
| `identity.preserves` | Identity resolution, published by daemon init | Public identity artifact; distinct from private endpoint material. |
| `identity-receipt.preserves` | Identity resolution, published by daemon init | Source selection, public endpoint, permission and rotation evidence; no secret bytes. |
| `startup-receipt.preserves` | Runtime startup through daemon | Startup decision and linked adapter evidence; can exist even when startup denies before lock installation. |
| `health-receipt.preserves` | Status and restart paths | Latest written local health observation; not an append-only history or process-liveness oracle. |
| `shutdown-receipt.preserves` | Admitted shutdown path | Shutdown evidence tied to startup; removed by the clean restart path after passing restart-health evaluation. |
| `status-control-receipt.preserves` | Direct status path | Control evidence for the local status request; status is therefore not read-only. |
| `stop-control-receipt.preserves` | Direct stop and denial paths | May record denial rather than completed shutdown. Read the decision and result bindings. |
| `control/node.lock.preserves` | Passing startup, removed after shutdown effects | Startup-bound control lock; existence alone is not proof of a live operating-system process. |

Optional CLI outputs are copies selected by the caller, not alternate roots used by the daemon. For example, `--startup-out`, `--health-out`, and `--shutdown-out` export their respective values, while status/stop `--receipt-out` exports control evidence. The [CLI declarations](../../../src/cli/ops/node/command/base.rs) and [lifecycle shell](../../../src/cli/ops/node/parts/lifecycle/p001/body.rs) own those options. Use distinct export names when preserving multiple observations.

## Namespace and store ownership

The physical mapping is defined by `NodeStateNamespaceKind::as_str` in [molten-node-host](../../../crates/molten-node-host/src/node/state/authority.rs).

| Capability view | Physical relative directory | Review boundary |
| --- | --- | --- |
| `Identity`, `Secrets` | `identity` | Secret creation and acquired-handle observation; shared physical mapping does not erase namespace identity. |
| `Ledger` | `ledger` | Nested artifact evidence storage through derived store authority. |
| `ControlInbox`, `ControlOutbox` | `control/inbox`, `control/outbox` | Pending entries versus archived requests and operation evidence. |
| `ControlIngress`, `Ingress` | `control/iroh-ingress` | Ingress artifacts; transport observation is not operation authorization. |
| `ControlIdempotency` | `control/idempotency` | Local idempotency state; not a global exactly-once guarantee. |
| `ControlService` | `control/service` | Service-loop state, including its separate service lock. |
| `Services`, `Receipts`, `Registry` | `services`, `receipts`, `registry` | Service state, lifecycle evidence, and registry-scoped content. |
| `Chunks`, `Storage` | `chunks`, `storage` | Narrower storage authority, not permission to reopen descendants ambiently. |

Layout creation additionally prepares `cache`, `remote-dataspace`, `jobs`, `coordination`, `plugin-host`, `catalog-mcp`, and `control`. Their existence only establishes layout preparation. It does not establish successful service startup or usable live adapters.

## Request-relative and adapter-relative artifacts

The [path helpers](../../../src/node/parts/daemon/p034/body.rs) derive names from a request-ref file stem. The following are naming patterns, not executable templates or fabricated references.

| Pattern | Meaning |
| --- | --- |
| `control/inbox/<stem>.preserves` | Pending canonical request. |
| `control/inbox/<stem>.queue-receipt.preserves` | Queue evidence associated with that request. |
| `control/outbox/<stem>.request.preserves` | Archived request after dispatch handling. |
| `control/outbox/<stem>.dispatch-receipt.preserves` | Dispatch queue evidence, distinct from operation results. |
| `control/outbox/<stem>.control-receipt.preserves` | Control-level decision and bindings. |
| `control/outbox/<stem>.operation-receipt.preserves` | Operation-specific evidence where produced. |
| `receipts/adapter-start-<name>.preserves` | Adapter startup evidence written before the root startup artifact. |
| `receipts/adapter-shutdown-<name>.preserves` | Adapter shutdown evidence produced by an admitted plan. |

Do not derive trust from a basename. Resolve the actual canonical value and compare its request/startup/result bindings. The endpoint namespace also contains `node-endpoint.secret` and `node-endpoint.id`, defined by the [identity implementation](../../../src/node/parts/identity/p000/body.rs). The former is secret material and must not enter ordinary diagnostic bundles.

## Worked inventory failure

Suppose an inventory finds startup, health, and a lock, but no shutdown. That fits the classifier's running combination only if configuration and identity receipt also exist. It does not prove a process is alive. A previous interruption can leave the same combination, and restart rejects an existing startup without clean shutdown evidence.

Conversely, a directory containing startup and shutdown but still containing the lock is inconsistent under the classifier. Do not normalize it by deleting files. Record each presence observation, artifact decision, canonical binding, and collection boundary. The compatibility inspection helper can return `Empty` when it cannot open the root, so distinguish inaccessible from genuinely absent state in operator reports.

Canonical identity comes from Preserves and BLAKE3, not host paths, in-memory pointer identity, Rust layout, or directory enumeration order. Scoped filesystem containment does not prove crash atomicity, distributed consistency, peer trust, or release readiness.

## Sources

- [Handbook](../README.md)
- [Node-state filesystem authority](../../node-state-filesystem-authority.md)
- [Capability-rooted state companion](../../technical/storage/capability-rooted-node-state.md)
- [Artifact constants](../../../src/node/parts/daemon/p000/body.rs)
- [Root and namespace ownership](../../../crates/molten-node-host/src/node/state/authority.rs)
- [Lifecycle classifier and restart](../../../src/node/parts/daemon/p036/body.rs)
- [Publication ordering](../../../src/node/parts/daemon/p018/body.rs)
