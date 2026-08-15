## Why

Molten's peering model must not accidentally become Raft membership. A peer can be discovered, admitted for gossip, connected over Iroh, or authorized for a scoped node-control operation without being safe to add as a control-plane voter. Future Raft/control-plane membership joins need stronger admission: peer/session evidence plus explicit authority, policy, resource, source-gate, state-machine compatibility, snapshot/replay readiness, and quorum-safety receipts.

## What Changes

- Define canonical Raft membership-change request, preflight, and receipt records.
- Require membership joins to pass a dedicated control-plane admission gate rather than relying on peer connected state.
- Add deterministic membership preflight checks for peer session scope, authority, policy, resources, source-gate/provenance, state-machine compatibility, snapshot/replay readiness, and quorum preservation.
- Keep ordinary peer bootstrap, gossip topic joins, docs joins, protocol sessions, and job pools separate from Raft voter membership.

## Impact

- **Files**: consensus membership specs/core, peer-bootstrap boundaries, node-control CLI/runbook surfaces, diagnostics, and positive/negative tests.
- **Testing**: positive non-mutating preflight fixtures and negative tests for connected-peer-only, missing authority, missing source-gate, incompatible state-machine, stale snapshot, and quorum-safety denial.
