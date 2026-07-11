## Context

Molten's current architecture has strong reusable components, but they are described through several domain stories: actor turns, dataspaces, plugins, jobs, coordination, Raft control state, Iroh exchange, and evidence workflows. The missing architectural invariant is which behavior belongs to the fabric and which belongs to a distributed service built on the fabric.

This change establishes that invariant before lower-level system-extension, transport, durability, membership, consistency, scheduling, and simulation changes widen runtime authority.

## Decisions

### 1. The fabric owns mechanisms; extensions own distributed-service semantics

**Choice:** Molten core owns typed ports, lifecycle, authority, resources, canonical envelopes, supervision, and deterministic execution boundaries. Extensions own semantics such as transaction isolation, log offsets, queue delivery, shard policy, workflow state, and application protocols.

**Rationale:** This prevents one reference workload from becoming hidden global policy and keeps the fabric useful for databases, logs, schedulers, storage systems, actor services, and future distributed runtimes.

### 2. Three extension tiers are explicit

**Choice:** Define sandboxed plugins, system extensions, and applications/workloads separately. System extensions are long-lived, strongly admitted services that may own protocols and durable state; ordinary plugins do not gain those powers implicitly.

**Rationale:** A database engine or replicated log needs a larger lifecycle and effect surface than a policy or transformation plugin, but widening every plugin would create ambient authority and an unreviewable hot path.

### 3. Capability ports are canonical and registered

**Choice:** Add canonical port descriptors keyed by stable port identity and version. Descriptors declare operation classes, schemas, authority/resource requirements, determinism and replay classes, implementation profile, and non-claims. Unknown, duplicate, incompatible, or silently substituted ports deny.

**Rationale:** Extensions need portable mechanism contracts without depending on Iroh, Redb, Wasmtime, or another adapter's internal types.

### 4. No global coordination mechanism is mandatory

**Choice:** Extensions select explicit consistency, transport, durability, and scheduling profiles. The fabric does not force all messages, state, or services through Raft, one dataspace, one global log, or one storage engine.

**Rationale:** Distributed systems require different trade-offs. Making one mechanism universal would harm scalability and incorrectly transfer its guarantees to unrelated workloads.

### 5. Reference systems are conformance witnesses, not product identity

**Choice:** Fabric sufficiency is demonstrated when a transactional key-value service, replicated log, and distributed scheduler can be implemented without node-core modifications or authority bypasses. Their domain semantics remain extension-owned.

**Rationale:** Three materially different systems expose missing general primitives more reliably than a single database benchmark.

### 6. Evidence stays at semantic and trust boundaries

**Choice:** Molten emits canonical evidence for admission, lifecycle, protocol epochs, durable boundaries, checkpoints, failures, and operator-visible state. Internal page reads, packets, and other hot-path operations may use bounded aggregate evidence rather than one heavyweight receipt per primitive operation.

**Rationale:** Evidence must remain useful without making the fabric unusable for high-throughput services.

### 7. No OpenRaft dependency or adaptation

**Choice:** Molten does not select, adapt, or use OpenRaft. Consensus laws, currentness, membership safety, command identity, snapshots, and state-machine transitions remain pure mechanism contracts; any live consistency engine is an optional reviewed system extension over explicit transport, durable-state, time, entropy, and evidence ports.

**Rationale:** A backend-specific leader, term, storage, transport, or Rust DTO model must not become canonical Molten authority or force unrelated traffic through one consensus implementation.

### 8. Aspen behavior is clean-room input only

**Choice:** Aspen `main` behavior may inform independently stated requirements and black-box tests, but AGPL-licensed implementation code, comments, and fixtures are not copied into dual-licensed Molten without an explicit compatible relicensing grant.

**Rationale:** Requirements and conformance observations can guide an independent implementation while preserving Molten's `MIT OR Apache-2.0` licensing boundary.

## Functional core / imperative shell split

- Pure core: validate fabric descriptors, extension tiers, port compatibility, profile selection, non-claims, and reference-system capability requirements from in-memory values.
- Shell: discover adapter implementations, construct admitted bindings, start services, perform I/O, persist receipts, and render operator summaries.

## Risks / Trade-offs

- A broad port catalog can become an unstable pseudo-standard. Version every port and add only behavior required by reviewed system extensions.
- A privileged system-extension tier increases attack surface. Keep it separately admitted, capability-scoped, and unavailable to ordinary plugins by default.
- Reference systems could be misread as compatibility claims. Require explicit non-claims and conformance scope on every reference receipt.
