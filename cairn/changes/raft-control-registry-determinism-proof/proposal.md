## Why

The control-plane registry is the representative replicated state machine. To trust coordination and other control-plane services, identical admitted command logs must produce identical registry state, duplicate client commands must not apply twice, and snapshots must restore exactly the same state evidence.

## What Changes

- Add proof requirements for deterministic control-registry apply over admitted Raft command logs.
- Require duplicate client-session and sequence-number replay evidence.
- Require snapshot/restore equivalence and stale/tampered snapshot denial evidence.

## Impact

- **Files**: Raft/control-plane runtime, command envelope tests, snapshot/restore tests.
- **Testing**: generated command logs, duplicate replay, divergent command denial, read-index freshness, and snapshot/restore round trips.
