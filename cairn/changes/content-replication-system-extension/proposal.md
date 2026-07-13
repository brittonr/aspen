## Why

Content storage and transfer are mechanisms, while replication factor, placement, repair, handoff, and convergence are workload policies. Putting those policies in the chunk primitive or Iroh adapter would make one replication strategy global and would couple local content identity to live network behavior.

Molten needs an optional system extension that owns replication semantics while consuming the fabric content, transport, durable-state, time, membership, placement, resource, and observability ports.

## What Changes

- Add a canonical content-replication system-extension manifest with explicit placement, replica, repair, retention, resource, and evidence profiles.
- Add pure deterministic inventory comparison, target selection, repair, handoff, and reconciliation plans.
- Execute bounded receiver-driven replication and repair through admitted content-store and transport adapters.
- Bind active transfers and replicas to service generation, operation ids, content refs, placement epochs, and retention pins.
- Model under-replication, unavailable peers, stale placement, uncertain delivery, corruption, cancellation, restart, and repair exhaustion explicitly.
- Provide deterministic simulation and live-loopback conformance without making replication part of ordinary blob access.

## Impact

- **Files**: a new content-replication extension, system-extension manifests, placement and repair primitives, content/transport adapter bindings, status assertions, operator readback, fixtures, and a new `content-replication` accepted spec.
- **Testing**: placement and reconciliation properties, partial replica recovery, stale-epoch fencing, corruption repair, duplicate operations, resource pressure, restart, retention, live/sim parity, and negative authority tests.
- **Safety**: replica count and repair receipts prove only bounded observed extension behavior; they do not prove permanent durability, global availability, confidentiality, exact-once transfer, or application correctness.
- **Licensing**: Aspen `main` replication architecture is a design reference; implementation must use compatible upstream code or independent source.
