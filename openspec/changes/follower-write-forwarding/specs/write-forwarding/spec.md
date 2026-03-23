## write-forwarding

### Requirements

1. When `RaftNode::write()` receives `ForwardToLeader` and a `WriteForwarder` is set, it MUST forward the write to the indicated leader
2. Forwarding MUST use the existing iroh QUIC connections (connection pool or direct connect)
3. If the forwarding target is also not the leader, return `NotLeader` — do not chain-forward
4. Forwarding MUST NOT hold any local locks across the network call
5. Forwarding timeout MUST be bounded (30s max)
6. When no `WriteForwarder` is set, behavior MUST be unchanged (return `NotLeader`)
7. The write batcher on the follower MUST be bypassed for forwarded writes
8. Worker stats KV write failures MUST log at DEBUG, not WARN

### Acceptance Criteria

- 3-node cluster survives leader change during long-running CI job: job ack succeeds on follower
- Worker stats writes succeed after leader change (forwarded to new leader)
- No infinite forwarding loops
- Unit test: `test_write_forwarding_on_leader_change`
- Integration test: re-run multi-node dogfood with forced leader change during build stage
