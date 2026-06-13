## Context

The artifact registry gives Molten stable ids and dependency graphs. Iroh provides blobs for immutable content and docs/gossip for mutable discovery and coordination surfaces. Effect manifests describe what an artifact may ask the runtime to do. Remote artifact sync ties these pieces together so an actor or peer can request execution by artifact id rather than relying on ambient predeployment.

Unison's model of sending bytecode references and syncing missing hashes is prior art. Molten's adaptation must be more explicit about trust boundaries: artifacts are not executable until admitted by local policy, capabilities are not inherited from the sender by default, and all remote side effects pass through handlers.

## Goals

- Sync missing artifact dependencies on demand using content-addressed ids.
- Verify fetched bytes against artifact ids before registry installation.
- Preserve local autonomy: each receiving peer applies its own policy before installing or executing an artifact.
- Cache verified artifacts for later use without changing identity.
- Bind remote execution to declared effects, target handler profile, and presented capabilities.
- Produce enough receipts to replay or audit how code and dependencies reached a peer.

## Non-Goals

- Do not support arbitrary closure serialization from a live Rust, Wasm, or Steel heap.
- Do not make remote sync a replacement for normal actor messages, choreography protocol messages, Raft commands, or blob transfer semantics.
- Do not allow a sender to force-install artifacts without receiver policy admission.
- Do not assume network connectivity; sync protocols must support missing, delayed, denied, or partial dependencies.
- Do not execute artifacts whose dependency closure is incomplete or whose required effects have no admitted handlers.

## Remote install protocol

A remote install or execution request should begin with a root artifact id and a closure descriptor:

- root artifact id,
- expected direct dependencies and optional closure hash,
- artifact kinds and size hints,
- required effect manifest ids,
- policy/evidence refs known to the sender,
- requested handler profile or acceptable profile set,
- replay/session nonce.

The receiver checks its local registry, computes missing ids, and requests missing payloads and metadata. Fetched artifacts are verified by canonical hash and staged. Only after all required dependencies are present and admitted does the receiver install the closure.

## Transport mapping

- Iroh blobs carry immutable artifact payload bytes and large canonical metadata.
- Iroh docs may publish mutable indexes, name metadata, peer cache hints, and availability records.
- Iroh gossip may announce small install/execution requests and response envelopes.

Transport messages are envelopes. The sync protocol never trusts transport-level identity alone; artifact ids and capabilities are validated at the Molten layer.

## Remote execution envelope

A remote execution request should carry:

- execution id,
- root artifact id,
- entrypoint or exported function name/id,
- argument value as canonical Preserves or content ref,
- dependency closure hash or install receipt ref,
- effect manifest ref,
- requested handler profile,
- presented capabilities and attenuations,
- caller identity and reply route,
- policy/evidence refs.

The target admits the request, binds handlers, starts execution in the appropriate adapter, and returns either a canonical result value/content ref or a structured failure receipt.

## Caching and garbage collection

Verified artifacts may be cached by id. Cache entries should record:

- source peer or blob ticket,
- verification result,
- local admission result,
- last use,
- dependent durable records or receipts,
- whether the artifact is pinned by policy, name metadata, storage records, or active sessions.

Garbage collection must not remove artifacts referenced by durable storage, receipts, active executions, installed protocols, or pinned metadata.

## Policy and evidence

Receipts should cover:

- missing-set calculation,
- fetch source and content id,
- canonical hash verification,
- dependency closure admission,
- artifact install admission,
- handler binding admission,
- execution admission,
- result or failure.

Nickel contracts cover static sync limits, allowed artifact kinds, size limits, schema requirements, and handler profile policy. Steel predicates may be used only for reviewed dynamic trust decisions. Trellis predicates can bound replay, dependency closure membership, content integrity, and routing constraints. Basalt enforces capabilities. Cairn validates receipts before they are evidence.

## Open Questions

- Should closure descriptors include the complete dependency set or allow iterative discovery from fetched artifact metadata?
- How should peers advertise artifact availability without leaking private artifact names or policies?
- What is the first remote execution target: native test adapter, Wasmtime component, or Steel script?
- Which cache eviction policy can be proven safe with respect to durable typed storage references?
