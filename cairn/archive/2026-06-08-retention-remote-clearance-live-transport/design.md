## Context

`retention-remote-clearance-live-workflow` introduced canonical request/response/import artifacts. The next step is to move those artifacts over the existing node-control live workflow surface instead of requiring manual file transfer. The retention import gate remains the local safety boundary.

## Decisions

### 1. Reuse node-control live workflow evidence boundaries

**Choice:** Add a retention-clearance live workflow on the node-control live path that binds request and response refs into canonical live workflow receipts.

**Rationale:** Node-control live workflow already models loopback send/receive, bundle import/export, ack, reconcile, and evidence binding without making transport identity authoritative.

### 2. Keep retention import as the deletion-safety gate

**Choice:** The live workflow returns a response artifact, but local deletion may use it only after `retention-remote-gc-clearance-import-v1` passes and stores the embedded peer clearance locally.

**Rationale:** This preserves the fail-closed behavior and avoids treating transport delivery, peer tickets, or live workflow receipts as authority or policy.

### 3. Make loopback deterministic before remote networking polish

**Choice:** The first transport implementation exposes a deterministic local/loopback CLI that exercises the same request/respond/import flow and stores live workflow receipts. A future slice can add multi-host ergonomics if needed.

**Rationale:** The loopback path gives reproducible tests for the live workflow semantics without depending on external network timing.

## Risks / Trade-offs

- Loopback transport is not yet an operator-friendly multi-host command sequence; it proves the node-control live receipt and import semantics first.
- Retention receiver evaluation is bounded by supplied local retained/revoked/stale inputs in this slice; richer remote reference-index discovery can be added later without changing import semantics.
