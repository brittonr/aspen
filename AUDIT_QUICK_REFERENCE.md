# Network & P2P Layer Audit - Quick Reference

## Critical Issues (Fix Before Production)

### Issue #1: No Message Framing (CRITICAL)

- **File**: `src/raft/network.rs` (lines 173-182), `src/raft/server.rs` (lines 150-158)
- **Problem**: Raw postcard serialization with no length prefix. Server uses `read_to_end()` which requires stream closure to detect message boundary.
- **Risk**: Resource exhaustion, slow-loris attacks, protocol deadlocks
- **Fix**: Implement 4-byte big-endian length prefix before each message

### Issue #2: Unbounded Stream Allocation (CRITICAL)

- **File**: `src/raft/server.rs` (lines 122-139)
- **Problem**: Server accepts unlimited bidirectional streams per connection with no limits
- **Risk**: DoS via stream explosion, memory exhaustion, scheduler starvation
- **Fix**: Add per-connection stream limit (e.g., 100 concurrent max)

### Issue #3: Missing Gossip Message Signature (CRITICAL)

- **File**: `src/cluster/gossip_discovery.rs` (lines 67-89, 171-238)
- **Problem**: `PeerAnnouncement` struct has NO signature field despite doc comments claiming signatures are used
- **Risk**: Impersonation attacks, man-in-the-middle, cluster poisoning
- **Fix**: Add signature field and cryptographically sign all announcements with endpoint secret key

### Issue #4: No Partition Detection (CRITICAL)

- **File**: `src/raft/node_failure_detection.rs` (lines 171-185)
- **Problem**: Cannot distinguish network partition from actual node crash. Both classified as "NodeCrash"
- **Risk**: Operator may promote learner during partition, creating split-brain
- **Fix**: Track duration since last contact, add "PartitionDetected" state distinct from "NodeCrash"

### Issue #5: No Peer Verification (CRITICAL)

- **File**: `src/cluster/gossip_discovery.rs` (lines 224-238)
- **Problem**: Network factory accepts peer additions from gossip without verifying node_id matches endpoint_id
- **Risk**: Eclipse attack, all traffic redirected to attacker
- **Fix**: Verify endpoint signature before adding to network factory

---

## High Priority Issues (Fix Before ≥3 Nodes)

### Issue #6: No Connection Pooling (HIGH)

- **File**: `src/raft/network.rs` (lines 149-182)
- **Problem**: Each RPC creates new connection, no pooling or reuse
- **Risk**: Resource exhaustion, performance degradation on large clusters
- **Fix**: Implement connection pool with per-peer limits and TTL

### Issue #7: No Read Timeout on Server (HIGH)

- **File**: `src/raft/server.rs` (lines 151-154)
- **Problem**: `recv.read_to_end()` has no timeout; peer can stall indefinitely
- **Risk**: Task hangs forever, accumulates with stream explosion issue
- **Fix**: Wrap with `tokio::time::timeout(IROH_READ_TIMEOUT, ...)`

### Issue #8: Potential Quorum Bypass (HIGH)

- **File**: `src/raft/learner_promotion.rs`
- **Problem**: Learner promotion safety checks exist but unclear if quorum verification is enforced
- **Risk**: Could promote learner when cluster lacks quorum
- **Fix**: Verify current cluster has quorum of existing voters before promotion

---

## Medium Priority Issues (Hardening)

### Issue #9: No Request/Response Matching

- **File**: `src/raft/rpc.rs`, `src/raft/network.rs` (lines 166-195)
- **Problem**: No request ID in protocol; matching relies on stream synchrony
- **Risk**: Silent data corruption if message ordering violated
- **Fix**: Add request_id field to request/response structs

### Issue #10: Timeout Configuration Issues

- **File**: `src/raft/constants.rs` (lines 43-51)
- **Problem**: IROH_READ_TIMEOUT (10s) applies to all RPC types; too long for heartbeats
- **Risk**: 10s heartbeat latency, slow partition detection
- **Fix**: Use different timeouts per RPC type (heartbeat: 2s, vote: 5s, snapshot: 60s)

### Issue #11: No Endpoint Shutdown Timeout

- **File**: `src/cluster/mod.rs` (lines 634-642)
- **Problem**: No timeout on `endpoint.close()` for gossip/router cleanup
- **Risk**: Hangs during shutdown, resource leaks
- **Fix**: Add timeout wrapper around endpoint.close()

### Issue #12: No Peer TTL or Deduplication

- **File**: `src/cluster/gossip_discovery.rs`
- **Problem**: Peer entries never expire, no rate limiting on announcements
- **Risk**: Stale peers remain in network factory, spam possible
- **Fix**: Implement peer table TTL and per-node announcement rate limiting

---

## Attack Scenarios

### Slow-Loris Memory Exhaustion

```
Attacker sends 1 byte per second on 100 streams × 10MB = 1GB total allocation
No timeout + no message framing = indefinite hang + memory exhaustion
```

### Impersonation via Gossip

```
Attacker broadcasts: {node_id: 1, endpoint_addr: attacker_ip}
No signature validation → all nodes trust announcement
Legitimate node 1 traffic gets redirected to attacker
```

### Split-Brain via Partition

```
Network partition splits cluster in half
NodeFailureDetector classifies all disconnected nodes as "NodeCrash" (not partition)
Operator promotes learner in minority partition
Partition heals: now two leaders, data inconsistency
```

---

## Testing Checklist

- [ ] Test stream explosion: 1000+ concurrent streams from single connection
- [ ] Test slow-loris: send 1 byte per 10 seconds on 100 streams
- [ ] Test gossip impersonation: broadcast fake announcements
- [ ] Test partition healing: partition cluster, promote learner, heal partition
- [ ] Test connection exhaustion: rapid connect/disconnect cycles
- [ ] Test message framing: send truncated messages, verify timeout

---

## File Locations Summary

| Category | File | Lines | Issue |
|----------|------|-------|-------|
| Framing | `src/raft/network.rs` | 173-182 | #1 |
| Framing | `src/raft/server.rs` | 150-158 | #1 |
| Streams | `src/raft/server.rs` | 122-139 | #2 |
| Timeouts | `src/raft/server.rs` | 151-154 | #7 |
| Gossip Auth | `src/cluster/gossip_discovery.rs` | 67-89 | #3 |
| Gossip Auth | `src/cluster/gossip_discovery.rs` | 171-238 | #3 |
| Peer Verify | `src/cluster/gossip_discovery.rs` | 224-238 | #5 |
| Pooling | `src/raft/network.rs` | 149-182 | #6 |
| Partition | `src/raft/node_failure_detection.rs` | 171-185 | #4 |
| Promotion | `src/raft/learner_promotion.rs` | 59-92 | #8 |
| RPC IDs | `src/raft/rpc.rs` | All | #9 |
| Timeouts | `src/raft/constants.rs` | 43-51 | #10 |
| Shutdown | `src/cluster/mod.rs` | 634-642 | #11 |
| TTL | `src/cluster/gossip_discovery.rs` | All | #12 |
