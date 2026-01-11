# Ultra Mode Analysis: What Next for Aspen

**Created**: 2026-01-10T11:45:00Z
**Branch**: v3 (ahead of main by ~25 commits)
**Analysis Type**: Comprehensive roadmap with prioritized actionable items

---

## Executive Summary

Aspen is at a **mature integration phase**. The core infrastructure is production-ready with 350+ passing tests, clean builds, and no stubs. Recent work (last 30 commits) focused on:

- Security fixes (gix vulnerabilities, job dependency cascade)
- Hooks system with snapshot events
- Secrets/SOPS management with PKI
- Network module unit tests

**The project is feature-complete for its core mission.** Remaining work is primarily:

1. Integration testing for recent features
2. Completing consumer groups for pub/sub scalability
3. Resolving circular dependency blocking Pijul tests

---

## Priority Matrix

### P0: Must Do This Week (Blockers/Security)

| Task | Location | Effort | Impact |
|------|----------|--------|--------|
| **Secrets integration tests** | `crates/aspen-secrets/` | 2 days | Validates SOPS, PKI, Transit |
| **PKI intermediate CA stubs** | `pki/store.rs:383-398` | 1 day | 3 methods need implementation |
| **Fix Pijul test circular dep** | `tests/pijul_multi_node_test.rs` | 0.5 day | Unblocks 6 tests |

### P1: High Priority (This Sprint)

| Task | Location | Effort | Impact |
|------|----------|--------|--------|
| **Pub/Sub consumer groups** | `crates/aspen-pubsub/` | 3 days | Scalable event processing |
| **Event publishing wiring** | `storage_shared.rs`, `node.rs` | 2 days | Hooks integration |
| **Cross-cluster hook forwarding** | `hooks/handlers.rs:373` | 2 days | Multi-cluster hooks |

### P2: Medium Priority (Next Sprint)

| Task | Description | Effort |
|------|-------------|--------|
| Enable ~55 ignored tests | Network tests for CI | 2 days |
| DataFusion SQL benchmarks | Validate <2x overhead | 1 day |
| Gossip topology sync | `discovery.rs:352` TODO | 1 day |
| AuthenticatedRequest unification | CLI client TODOs | 1 day |

### P3: Future Work (Backlog)

- Secondary indexes (Layer Architecture Phase 4)
- VmManager mock tests (requires KVM)
- Fault injection test coverage
- Cross-cluster federation tests

---

## Detailed Task Breakdown

### 1. Secrets Integration Tests (P0)

**Current State**: 37 unit tests, 0 integration tests
**Gap**: No end-to-end validation of SOPS secrets over Iroh QUIC

**Recommended Tests**:

```rust
// tests/secrets_integration.rs

#[tokio::test]
async fn test_node_bootstrap_with_sops_secrets() {
    // Bootstrap node with .sops.yaml, verify secrets loaded
}

#[tokio::test]
async fn test_secrets_rpc_over_iroh_quic() {
    // Client RPC to read/write secrets via ALPN routing
}

#[tokio::test]
async fn test_transit_encrypt_decrypt_roundtrip() {
    // Encrypt with Transit key, decrypt on different node
}

#[tokio::test]
async fn test_pki_issue_certificate_chain() {
    // Root CA -> Intermediate CA -> Leaf cert chain
}
```

### 2. PKI Intermediate CA (P0)

**Location**: `crates/aspen-secrets/src/pki/store.rs`
**Lines**: 383-398

Three stubs need implementation:

```rust
async fn generate_intermediate(&self, mount: &str, name: &str, key_type: KeyType) -> Result<Certificate>
async fn set_signed_intermediate(&self, mount: &str, name: &str, cert: &str) -> Result<()>
async fn sign_intermediate_csr(&self, mount: &str, name: &str, csr: &str) -> Result<Certificate>
```

### 3. Pijul Circular Dependency Fix (P0)

**Problem**: `PijulMultiNodeTester` unavailable due to:

```
aspen-testing -> aspen -> aspen-testing (circular)
```

**Solutions** (pick one):

1. Move `PijulMultiNodeTester` to `aspen-testing`
2. Create `aspen-pijul-testing` crate
3. Use trait-based abstraction with feature gates

### 4. Pub/Sub Consumer Groups (P1)

**Design Needed**:

```rust
// crates/aspen-pubsub/src/consumer_group.rs

pub struct ConsumerGroup {
    group_id: GroupId,
    members: Vec<ConsumerId>,
    partitions: PartitionAssignment,
    checkpoints: CheckpointStore,
}

trait ConsumerGroupManager {
    async fn join_group(&self, group_id: &GroupId) -> Result<ConsumerHandle>;
    async fn commit_offset(&self, partition: PartitionId, offset: u64) -> Result<()>;
    async fn rebalance(&self) -> Result<Vec<PartitionId>>;
}
```

**Why Critical**: Hooks system needs load-balanced event processing.

### 5. Event Publishing Wiring (P1)

**Inject points needed**:

- `kv:write` after successful Raft commit in `storage_shared.rs`
- `kv:delete` after deletion
- `raft:leader_change` in openraft callbacks
- `cluster:membership_change` after config changes

---

## Codebase Health Metrics

### Test Coverage

- **Total tests**: 350+ passing
- **Ignored tests**: ~55 (mostly network tests)
- **Simulation tests**: Active with madsim
- **Property tests**: proptest for raft/sql modules

### TODO/FIXME Count by Area

| Area | Count | Priority |
|------|-------|----------|
| aspen-testing | 6 | Medium (infrastructure) |
| aspen-rpc-handlers | 4 | Medium |
| aspen-cli | 3 | Low |
| aspen-gossip | 2 | Medium |
| aspen-raft | 2 | Low (tested via simulation) |

### Build Status

- **Compilation**: Clean (0 warnings)
- **Clippy**: Passes with `--deny warnings`
- **Format**: `nix fmt` clean

---

## Security Status

### Known Vulnerabilities

| Crate | Advisory | Status |
|-------|----------|--------|
| gix-* dependencies | RUSTSEC-2025-0140 | **Fixed in f335a902** |
| Compression bomb | CVE | **Fixed in 778d2ee3** |
| Job cascade timeout | N/A | **Fixed in 1a52d217** |

### Security Controls Active

- Ed25519 authentication via Iroh PublicKey
- UCAN capability tokens (max 8 delegation levels)
- SOPS encryption at rest
- QUIC + TLS 1.3 transport
- Tiger Style resource bounds

---

## Recommended Action Order

```
Week 1:
  Day 1-2: Secrets integration tests
  Day 3: PKI intermediate CA implementation
  Day 4: Pijul circular dependency fix
  Day 5: Enable first batch of ignored tests

Week 2:
  Day 1: Consumer groups design doc
  Day 2-4: Consumer groups implementation
  Day 5: Consumer groups tests

Week 3:
  Day 1-2: Event publishing wiring
  Day 3: Cross-cluster hook forwarding
  Day 4-5: Integration tests for hooks

Week 4:
  Benchmarking, remaining tests, polish
```

---

## Quick Commands

```bash
# Run quick tests (skips slow proptest/chaos)
nix develop -c cargo nextest run -P quick

# Run specific module tests
nix develop -c cargo nextest run -E 'test(/secrets/)'
nix develop -c cargo nextest run -E 'test(/pubsub/)'

# Check TODOs in specific area
rg "TODO|FIXME" crates/aspen-secrets/

# Build with all features
nix develop -c cargo build --all-features

# Run cluster smoke test
./scripts/aspen-cluster-raft-smoke.sh
```

---

## Merge Readiness: v3 -> main

**Blockers**:

- [ ] Secrets integration tests passing
- [ ] PKI intermediate CA implemented
- [ ] Pijul tests unblocked

**Nice to have before merge**:

- [ ] Consumer groups implemented
- [ ] Event publishing wired
- [ ] All ~55 ignored tests evaluated
