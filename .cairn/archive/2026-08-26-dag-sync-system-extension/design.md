## Context

Several Molten subsystems traverse dependency graphs, but their current synchronization shells are local and domain-specific. A reusable protocol needs generic graph identity and traversal while leaving job scheduling, artifact admission, commit semantics, and merge policy to their owning extensions.

## Decisions

### 1. Graph and traversal laws are primitives

**Choice:** Canonical DAG nodes expose node ref, schema, ordered or canonicalized edge refs, metadata refs, and content refs. Pure traversal validates bounds, cycles, duplicates, order, visited state, and missing sets.

**Rationale:** Graph correctness should not depend on transport, storage, or a particular workload.

### 2. Synchronization is a system extension

**Choice:** A separately admitted extension owns peer sessions, inventory exchange, traversal strategy, resumable progress, retry, cancellation, and status. Domain extensions submit roots and policy; they do not receive raw transport handles.

**Rationale:** Network synchronization is optional long-lived behavior with authority and resource requirements.

### 3. The receiver owns the fetch plan

**Choice:** The receiver computes requested node and content refs from local verified state. Responses must match the active plan and traversal epoch; unsolicited or excess nodes are denied or ignored without indexing them as verified.

**Rationale:** Sender-driven graph expansion permits resource amplification and unreviewed import.

### 4. Traversal strategies are explicit and deterministic

**Choice:** Profiles may select full, stem-first, leaf-only, resumable missing-ref, or peer-partitioned traversal. Ordering, peer assignment, bounds, and completion criteria are canonical inputs.

**Rationale:** Strategy affects resource use and replay identity and cannot remain an implicit optimization.

### 5. Bytes move through content adapters

**Choice:** DAG protocol messages carry bounded metadata and content refs. Large node bodies and payloads use content-store/exchange ports and are verified before graph state advances.

**Rationale:** The graph protocol should not duplicate streaming, range, retention, or confidentiality behavior.

### 6. Domain admission follows synchronization

**Choice:** A completed sync receipt proves requested refs were received and verified. Installation, execution, merge, publication, or registry mutation requires the domain extension's normal admission gates.

**Rationale:** Availability is not semantic trust.

## Functional core / imperative shell split

- Pure core: node validation, cycle and bound checks, traversal order, missing-set and strategy plans, response matching, resume transitions, peer partitioning, and receipt payloads.
- Shell: query stores, open transport sessions, fetch content, persist partial progress, enforce cancellation/deadlines, and publish bounded status/evidence.

## Dependencies

- System-extension runtime.
- Fabric transport, content-store, durable-state, time, identity, resource, observability, and simulation profiles.

## Risks / Trade-offs

- Generic node schemas can become too abstract. Keep the base descriptor small and allow versioned domain metadata.
- Peer partitioning can duplicate work after failure. Bind assignments to traversal epochs and verify local availability before retry.
- Large or adversarial graphs can exhaust resources. Enforce hard node, edge, depth, byte, peer, and step limits.
