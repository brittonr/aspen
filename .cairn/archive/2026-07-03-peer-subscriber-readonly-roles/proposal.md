## Why

Read-only peers and subscribers are not just weaker writers. They are separate roles that still require explicit read authority, egress policy, redaction, resource bounds, replay boundaries, and revocation. Molten should let operators admit a peer to observe a topic, docs namespace, inventory feed, service status, or evidence stream without granting publish, assert/retract, node-control, job execution, retention, sync-import, or Raft membership authority.

## What Changes

- Define subscriber/read-only peer roles as attenuated peer-session capabilities with explicit scopes, expiry, revocation, resource limits, and egress policy refs.
- Add canonical subscription grants and projection receipts for read-only delivery, including redaction/filter decisions and replayability metadata.
- Require every subscriber surface to deny write, publish, assert, retract, execute, import, destructive, or authority-delegating operations unless a separate matching grant exists.
- Clarify that read-only peering is not Raft learner/non-voter membership and cannot serve linearizable control-plane reads without explicit read-index/read-capability evidence.

## Impact

- **Files**: peer/session role model, eventual surface projection receipts, node-control/readback diagnostics, federation/read-only sync boundaries, Raft membership docs, and positive/negative tests.
- **Testing**: positive subscriber delivery/readback tests and negative tests for missing read authority, egress policy denial, write upgrade attempts, stale subscription grants, unauthorized republish, sync-import from read-only hints, and Raft learner confusion.
