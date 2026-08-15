## Context

The chunk store already defines Molten object identity and verification. The missing layer is an adapter-neutral live byte substrate. Current local exchange helpers conflate an Iroh-shaped ticket with copying files between roots, which cannot establish live transport, streaming, cancellation, or backend recovery behavior.

## Decisions

### 1. Molten manifests remain canonical identity

**Choice:** Content adapters operate on canonical manifest and chunk refs. Backend blob hashes, tickets, tags, object keys, paths, and provider ids are hints bound in receipts but never replace the Molten ref.

**Rationale:** Changing storage or transport must not change object identity or admission semantics.

### 2. Content storage and content exchange are narrow ports

**Choice:** Define separate operations for local byte storage/indexing and remote content exchange. Profiles may implement both, but extensions request only put, get, verified range, availability, import/export, or protection operations they need.

**Rationale:** A universal backend API would leak Iroh or filesystem behavior into pure chunk and replication logic.

### 3. Streaming is bounded and verification-led

**Choice:** Put, fetch, and range operations declare byte, chunk, concurrency, memory, deadline, and cancellation bounds. Adapters emit incremental observations; pure verification decides whether each chunk and final manifest may become available or exposed.

**Rationale:** Large content must not require unbounded buffering, and an adapter success callback cannot substitute for hash and length validation.

### 4. Protection is not retention authority

**Choice:** Backend tags or leases may implement an admitted protection handle, but canonical retention pins and deletion gates remain authoritative. Unprotecting a backend object does not authorize deletion, and pinning does not grant read or reveal authority.

**Rationale:** Backend GC mechanisms and Molten retention policy have different trust scopes.

### 5. Completion and uncertainty are explicit

**Choice:** Adapter outcomes distinguish accepted, streaming, verified, durable where supported, cancelled, retryable, failed, and uncertain. Partial state records exactly which chunks were verified so resume plans remain deterministic.

**Rationale:** Disconnects and process failures cannot be safely normalized to success or definite absence.

### 6. Live and simulation profiles share conformance

**Choice:** Capability-rooted filesystem/Redb, live Iroh blob, and deterministic simulated adapters implement the same canonical port operations and observable state machine. Adapter-specific features are versioned optional capabilities.

**Rationale:** Replication and DAG protocol cores must run unchanged across live and simulated worlds.

## Functional core / imperative shell split

- Pure core: manifest parsing, chunk/range planning, missing-set calculation, hash/length/order verification, partial-state transitions, protection/retention decisions, completion classification, and receipt payloads.
- Shell: open capability roots and Redb handles, stream bytes, call Iroh blob APIs, manage backend tickets/tags, enforce cancellation, and persist admitted index/evidence updates.

## Dependencies

- Capability-rooted store threading.
- Fabric durable-state and transport ports.
- Cryptographic endpoint identity adapters for live Iroh profiles.

## Risks / Trade-offs

- Backend capabilities may not align exactly. Unsupported range, durability, or protection operations deny rather than silently emulate stronger guarantees.
- Streaming verification increases state-machine complexity. Keep chunk state explicit and bounded.
- Iroh API versions may change. Contain them entirely in the live adapter shell.
