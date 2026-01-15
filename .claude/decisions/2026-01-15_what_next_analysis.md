# What Next: Aspen Development Priorities

**Created**: 2026-01-15 (Ultra Mode Analysis)
**Branch**: v3 (300+ commits ahead of main)
**Current State**: Production-ready core, several 90-95% complete feature branches

---

## Executive Summary

Aspen is at a **finishing point** - the core is solid, but recent features need integration wiring to be complete. The most recent work (blob replication) is 95% done but needs final RPC handler wiring.

---

## Priority 1: Complete Blob Replication (IMMEDIATE)

**Status**: 95% complete, last commit was `04ba4103` (today)
**Blocker**: `handle_trigger_blob_replication()` at blob.rs:1259 returns placeholder

### What's Done
- `BlobReplicationManager` - Full implementation with configurable replication factor (1-7)
- `BlobReplicationRpc` - End-to-end RPC protocol implementation
- `BlobReplicatePull` handler - Target-side blob download working
- Automatic replication - Works when blobs are added

### What's Missing
- Wire `TriggerBlobReplication` RPC to `BlobReplicationManager`
- Integration tests for cross-node blob transfer
- CLI `blob repair` command for manual repair trigger

### Effort
~2-4 hours to wire, 1 day for tests

---

## Priority 2: Event Publishing Injection

**Status**: Hook system ready, not wired to core operations
**Impact**: Hooks are useless without events being published

### Events to Wire
```rust
// In crates/aspen-raft/src/storage_shared.rs and node.rs
"kv:write"      // After successful KV write
"kv:delete"     // After successful KV delete
"kv:batch"      // After batch operations
"raft:leader_change"  // When leadership changes
"raft:snapshot"       // After snapshot completion
```

### Effort
2-3 days

---

## Priority 3: Secrets Integration Tests

**Status**: 37 unit tests, NO integration tests
**Risk**: SOPS system untested over Iroh QUIC

### Required Tests
1. Node bootstrap with SOPS secrets file
2. Secret retrieval over Iroh RPC
3. Transit encryption roundtrip multi-node
4. PKI certificate issuance with roles

### Effort
2-3 days

---

## Priority 4: PKI Intermediate CA

**Status**: Methods at `pki/store.rs:383-398` return "not implemented"

### Methods Needed
```rust
async fn generate_intermediate(&self, ...) -> Result<Certificate>
async fn set_signed_intermediate(&self, ...) -> Result<()>
async fn sign_intermediate_csr(&self, ...) -> Result<Certificate>
```

### Effort
1-2 days

---

## Priority 5: Pub/Sub Consumer Groups

**Status**: Designed, not implemented
**Impact**: Blocks scalable hook handler distribution

### Effort
3-4 days implementation + tests

---

## Quick Wins (< 1 day each)

1. **CLI tag signing** - `--key` flag at `aspen-cli/commands/tag.rs:112`
2. **Gossip topology sync** - Compare versions at `discovery.rs:352`
3. **Event schema migration** - Upcasting at `event_store.rs:596`
4. **Blob repair CLI** - Expose `RunRepairCycle` command

---

## Technical Debt Summary

| Category | Count | Impact |
|----------|-------|--------|
| Critical Infrastructure | 4 | High |
| Test Coverage Gaps | 7 | Medium |
| Code Organization | 3 | Low |
| Nice-to-Have | 11 | Low |

---

## Recommended Order

### This Session
1. Wire `TriggerBlobReplication` RPC to complete blob replication
2. Add blob replication integration test

### This Week
3. Implement PKI intermediate CA operations
4. Add secrets integration tests
5. Wire event publishing to KV operations

### Next Week
6. Implement Pub/Sub consumer groups
7. Enable ignored integration tests
8. Security audit (upgrade ed25519-dalek)

---

## v3 -> main Merge Checklist

- [ ] Blob replication fully wired and tested
- [ ] Secrets integration tests pass
- [ ] PKI intermediate CA works
- [ ] Event publishing wired
- [ ] No new clippy warnings
- [ ] All tests pass

---

## Conclusion

**Recommended immediate action**: Wire `TriggerBlobReplication` to `BlobReplicationManager` - this completes the most recent feature work and is only ~2-4 hours of implementation.

The project is in excellent shape with 350+ tests passing. Most remaining work is integration wiring rather than new architecture.
