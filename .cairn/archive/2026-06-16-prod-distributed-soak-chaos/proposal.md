## Why

Molten's strongest evidence is deterministic local, loopback, and dogfood validation. Real production use depends on multi-node behavior across live Iroh transport, node-control workflows, coordination services, job workers, protocol gates, and retention/provenance boundaries over time. Those paths need soak and fault evidence before broad production use.

## What Changes

- Define a production-soak harness for live multi-node workflows.
- Add chaos/fault scenarios for network partition, duplicate delivery, stale tickets, restart, queue pressure, slow peer, corrupted artifacts, and partial retention state.
- Require canonical receipts, replay artifacts where possible, and explicit non-replayable diagnostics where live behavior cannot be replayed.
- Track performance/resource envelopes for queue depth, delivery latency, receipt growth, store growth, and recovery time.

## Impact

This change turns live distributed behavior from “smoke tested” into “soaked with bounded evidence.” It remains scoped: soak evidence supports a pilot decision but does not replace authority, provenance, policy, retention, source-gate, or destructive-operation gates.
