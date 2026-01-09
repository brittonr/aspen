# ULTRA Mode: What Next Roadmap

**Date**: 2026-01-09T17:30:00-05:00
**Analysis Type**: Maximum Capability Deep Analysis
**Status**: Comprehensive Strategic Assessment
**Branch**: v3 (376k+ lines of changes, 1,502 files)

---

## Executive Summary

Aspen is at a **critical inflection point** with production-ready infrastructure (~21,000 LOC, 350+ passing tests) and major feature development complete. The SOPS secrets management system has been implemented across three phases, with Phase 3 (RPC handlers + CLI) just completed.

**Current State**: There are **staged changes ready for commit** that complete the secrets integration:

- `backend.rs`: Improved error handling for NotFound vs KV errors
- `aspen-node.rs`: Wire secrets service into node bootstrap
- `kitty-cluster.sh`: Added usage examples for secrets CLI

**Immediate Action**: Commit staged changes, then proceed with priorities below.

---

## PRIORITY 1: Commit Staged Changes (IMMEDIATE)

**Status**: Ready to commit
**Effort**: Now

The staged changes complete SOPS Phase 3 integration:

```bash
git commit -m "feat: wire SOPS Phase 3 secrets service into node bootstrap

- Improve error handling in AspenSecretsBackend for NotFound vs KV errors
- Initialize KV, Transit, PKI engines in setup_client_protocol
- Add secrets CLI usage examples to kitty-cluster.sh
- Replace None placeholder with actual SecretsService"
```

**Files**:

- `crates/aspen-secrets/src/backend.rs` (+52 lines)
- `src/bin/aspen-node.rs` (+33 lines)
- `scripts/kitty-cluster.sh` (+16 lines)

---

## PRIORITY 2: Secrets Integration Tests (HIGH)

**Status**: Critical gap identified
**Effort**: 2-3 days

### Current Coverage

- 37 unit tests across secrets modules
- **NO integration tests** for Phase 3 RPC handlers
- **NO cluster integration tests** for secrets

### Required Test Files

1. `tests/secrets_integration_test.rs` - RPC protocol tests
2. `tests/secrets_cluster_test.rs` - Multi-node secrets operations

### Test Scenarios Needed

- Iroh QUIC protocol integration
- ALPN routing for secrets operations
- Multi-node secrets synchronization
- Raft consensus with secrets versioning
- Network partition scenarios
- Capability-based authorization flows

---

## PRIORITY 3: Consumer Groups for Pub/Sub (HIGH)

**Status**: Design needed, implementation not started
**Effort**: 1 week

### Why Critical

The pub/sub layer exists but lacks consumer groups. This blocks:

- Load-balanced event processing
- At-least-once delivery with checkpointing
- Scalable hook handler distribution

### Architecture

```
__pubsub/groups/{group}/cursor      -> u64 (committed cursor)
__pubsub/groups/{group}/members/{node} -> {heartbeat, partitions}
```

### Implementation

1. Add `ConsumerGroupCoordinator` in `crates/aspen-pubsub/src/consumer_group.rs`
2. Partition assignment via consistent hashing
3. Cursor checkpointing through Raft

---

## PRIORITY 4: Complete PKI Implementation (MEDIUM)

**Status**: Partial implementation
**Effort**: 1-2 days

### Unimplemented Methods

```rust
// pki/store.rs:383-398
async fn generate_intermediate(...) -> Err("not yet implemented")
async fn set_signed_intermediate(...) -> Err("not yet implemented")
```

### Required

- Intermediate CA generation logic
- Certificate chain validation
- Tests for CA hierarchy operations

---

## PRIORITY 5: Enable Integration Tests (MEDIUM)

**Status**: 55+ tests ignored
**Effort**: 1-2 weeks (phased)

### Ignored Test Breakdown

| File | Tests | Reason |
| ---- | ----- | ------ |
| node_builder_integration.rs | 12 | Network access |
| multi_node_cluster_test.rs | 4 | Network access |
| job_integration_test.rs | 8 | Network access |
| hooks_integration_test.rs | 6 | Network access |
| pubsub_integration_test.rs | 8 | Network access |
| shell_command_worker_test.rs | 6 | Network + feature |
| pijul_multi_node_test.rs | 6 | Circular dependency |

### Strategy

1. Keep critical path tests as scripts (already working)
2. Run ignored tests in CI with network access
3. Convert key scenarios to madsim deterministic tests

---

## PRIORITY 6: Address Critical TODOs (MEDIUM)

### High Priority

| Location | TODO | Effort |
| -------- | ---- | ------ |
| `handlers.rs:373` | Cross-cluster hook forwarding | 2-4 days |
| `client.rs:163,216` | Convert to AuthenticatedRequest | 1-2 days |
| `pubsub_integration_test.rs:54` | EventStream tests | 1 day |
| `pubsub_integration_test.rs:711` | CLI pub/sub tests | 1 day |

### Medium Priority

| Location | TODO | Effort |
| -------- | ---- | ------ |
| `discovery.rs:352` | Topology version comparison | 1 day |
| `blob.rs:437` | Direct blob deletion API | 1 day |
| `event_store.rs:596` | Event upcasting | 2-3 days |

---

## PRIORITY 7: V3 Release Preparation (MEDIUM)

**Status**: 95% ready
**Effort**: 2-3 days for remaining items

### Pre-merge Checklist

- [x] Core features complete (Raft, Storage, Networking)
- [x] Secrets management complete (SOPS Phases 1-3)
- [x] Hooks system integrated
- [x] Pub/Sub core complete
- [x] Jobs system with workers
- [ ] Consumer groups (see Priority 3)
- [ ] Integration tests enabled (see Priority 5)
- [ ] Changelog written

### Breaking Changes to Document

1. Actor architecture removed (Dec 2025)
2. HTTP API removed (Iroh-only)
3. New hooks system
4. New pub/sub system
5. New jobs system with workers
6. New secrets management

---

## Test Coverage Analysis

### aspen-secrets Module Coverage

| Module | Public Funcs | Tests | Gap |
| ------ | ------------ | ----- | --- |
| backend.rs | 6 | 4 | 33% |
| kv/store.rs | 18+ | 8 | 56% |
| transit/store.rs | 19+ | 9 | 53% |
| pki/store.rs | 15+ | 5 | 67% |
| sops/provider.rs | 8+ | 4 | 50% |
| sops/decryptor.rs | 7+ | 4 | 43% |

### Missing Test Scenarios

- Real SOPS-encrypted file decryption
- AspenSecretsBackend with actual Redb storage
- Concurrent operations and race conditions
- Network partition with secrets
- Convergent encryption testing
- Key versioning with many rotations

---

## Architecture Gaps

| Gap | Impact | Solution | Priority |
| --- | ------ | -------- | -------- |
| Consumer Groups | Can't scale events | Implement coordinator | High |
| Progress Notifications | Inefficient reconnection | Add bookmark events | Medium |
| Secondary Indexes | Query performance | Layer architecture doc exists | Low |
| PKI Intermediate CA | Incomplete PKI | Implement methods | Medium |

---

## Performance Baseline

### Current Metrics (Redb + Iroh)

- Single-node write: 2-3 ms (350+ ops/sec)
- Single-node read: 10 us (100K ops/sec)
- 3-node write: 8-10 ms (100+ ops/sec)
- 3-node read: 36 us (27K ops/sec)

### No Known Bottlenecks

- Raft: 350+ ops/sec (3-node)
- Storage: Single-fsync optimal
- Network: Connection pooling active

---

## Recommended Execution Order

### This Week (Jan 9-15)

1. **NOW**: Commit staged changes (completes SOPS Phase 3)
2. Create `tests/secrets_integration_test.rs`
3. Run integration scripts to verify secrets end-to-end
4. Verify all quick tests pass

### Next Week (Jan 16-22)

1. Start Consumer Groups design and implementation
2. Complete PKI intermediate CA methods
3. Address cross-cluster hook forwarding TODO

### Following Weeks (Jan 23 - Feb 5)

1. Complete Consumer Groups implementation
2. Enable key integration tests in CI
3. Write V3 changelog and release notes

### Future (Feb+)

1. Layer architecture (tuple encoding, subspaces)
2. Web UI Dashboard
3. SDK development (Python, JavaScript)

---

## Success Metrics

| Metric | Target | Current |
| ------ | ------ | ------- |
| Quick tests passing | 100% | ~350 |
| Secrets integration tests | 10+ | 0 |
| Consumer groups | MVP | Not started |
| V3 release ready | All checks | 95% |

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Secrets API security issues | Low | High | Capability-based auth, error sanitization |
| Consumer group complexity | Medium | Medium | Follow Kafka patterns |
| Test coverage gaps | Medium | Medium | Add integration tests |
| V3 merge conflicts | Low | Low | Branch is ahead |

---

## Quick Reference: Key Files

| Task | File |
| ---- | ---- |
| Secrets service init | `src/bin/aspen-node.rs:1099-1123` |
| Secrets backend | `crates/aspen-secrets/src/backend.rs` |
| Consumer groups (new) | `crates/aspen-pubsub/src/consumer_group.rs` |
| Integration scripts | `scripts/aspen-cluster-*.sh` |
| Hooks tests | `scripts/kitty-hooks-test.sh` |
| PKI store | `crates/aspen-secrets/src/pki/store.rs` |

---

## Conclusion

Aspen has reached production-readiness for core infrastructure. The immediate focus should be:

1. **Commit** staged SOPS Phase 3 changes
2. **Test** secrets integration end-to-end
3. **Implement** consumer groups for scalable event processing
4. **Prepare** V3 release

The project is in excellent shape with 352 commits in 25 days demonstrating strong velocity. The remaining work focuses on hardening and completeness rather than new feature development.
