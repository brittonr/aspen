# Aspen vs FoundationDB vs etcd: Gap Analysis

**Date**: 2025-12-22
**Analysis Type**: Feature comparison and gap identification

## Executive Summary

Aspen is a production-ready distributed key-value store with ~21,000 LOC and 350+ tests. While it implements solid foundations (Raft consensus, ACID storage, linearizable operations), it falls short of FoundationDB and etcd in several critical areas that may limit adoption for demanding distributed systems use cases.

---

## Feature Comparison Matrix

| Feature | Aspen | FoundationDB | etcd |
|---------|-------|--------------|------|
| **Consensus** | Raft (openraft) | Paxos-based | Raft |
| **ACID Transactions** | Single-key CAS, conditional batches | Full multi-key ACID | Mini-transactions |
| **MVCC** | Per-key version counters | True MVCC (5s window) | MVCC with revision history |
| **Conflict Detection** | CAS-based | OCC with read/write sets | Revision-based |
| **Watch/Subscribe** | Log streaming | Key-range watches | Key-range watches |
| **Leases/TTL** | Basic TTL support | No native support | Full lease system |
| **Secondary Indexes** | None | Via Record Layer | None |
| **Horizontal Scaling** | Single cluster | Unbounded sharding | Limited (single leader) |
| **Testing Framework** | madsim simulation | Industry-leading DST | Robustness testing |
| **Client SDKs** | Rust only | Python, Go, Java, C, etc. | Go, Python, Java, etc. |
| **Production Usage** | Experimental | Apple, Snowflake | Kubernetes backbone |

---

## Critical Gaps vs FoundationDB

### 1. **Transaction Model** (SEVERITY: HIGH)

**FoundationDB:**

- True MVCC with 5-second transaction window
- Optimistic Concurrency Control (OCC) with automatic conflict detection
- Transactions can span any number of keys across any shards
- Conflict ranges can be explicitly specified for fine-grained control
- Automatic retry with conflict detection feedback

**Aspen:**

- Per-key version counters (not true MVCC)
- CAS operations and conditional batches (up to 100 keys)
- No automatic conflict detection across key ranges
- No read-your-writes guarantee across batches
- Transactions are etcd-style If/Then/Else, not FDB-style

**Gap Impact:**

- Cannot implement complex transactional workflows
- No support for read-modify-write patterns on key ranges
- Application must implement retry logic manually
- Cannot build higher-level abstractions (like Record Layer)

**Remediation Priority:** HIGH

```
Estimated complexity: 3-6 months
Dependencies: Storage layer changes, consensus integration
```

### 2. **Layer Architecture** (SEVERITY: HIGH)

**FoundationDB:**

- Core ordered KV with transactions enables layers on top
- Record Layer provides: schema management, secondary indexes, query planning
- Document Layer: MongoDB-compatible API
- SQL Layer: JDBC connectivity
- Layers are stateless, horizontally scalable

**Aspen:**

- No layer abstraction
- Single data model (string keys/values)
- No secondary indexes
- No query planning
- SQL is via raw SQLite (not distributed)

**Gap Impact:**

- Cannot model complex data relationships
- No path to SQL or document database compatibility
- Users must implement their own indexing
- Limited to simple key-value use cases

**Remediation Priority:** HIGH (but depends on transaction model)

### 3. **Horizontal Scalability** (SEVERITY: HIGH)

**FoundationDB:**

- Unbounded horizontal scaling via sharding
- Automatic data distribution and rebalancing
- Separate scaling of reads (storage servers) and writes (transaction system)
- Can handle petabytes of data
- f+1 replicas for f failures (not 2f+1)

**Aspen:**

- Single Raft cluster (no sharding)
- All data on all nodes
- MAX_PEERS = 1,000 (practical limit much lower)
- MAX_SNAPSHOT_SIZE = 100MB
- Limited by single-leader write throughput

**Gap Impact:**

- Cannot scale beyond ~10 nodes effectively
- Total data limited to what fits in SQLite on each node
- Write throughput limited by single Raft leader
- Not suitable for large-scale deployments

**Remediation Priority:** HIGH

### 4. **Deterministic Simulation Testing** (SEVERITY: MEDIUM)

**FoundationDB:**

- Single-threaded deterministic simulation of entire cluster
- Simulates: network, disk, machines, datacenters
- BUGGIFY macros inject failures at code level
- Reproducible with seed
- "Trillion CPU-hours equivalent" of testing
- Result: Almost zero customer-reported bugs

**Aspen:**

- madsim-based simulation (good foundation)
- Does not simulate disk failures
- No BUGGIFY-style code-level fault injection
- Less comprehensive than FDB
- Simulation artifacts captured, but coverage is narrower

**Gap Impact:**

- Lower confidence in edge case handling
- May miss subtle distributed bugs
- Recovery paths less thoroughly tested

**Remediation Priority:** MEDIUM (madsim is a solid base to build on)

### 5. **Client SDK Ecosystem** (SEVERITY: MEDIUM)

**FoundationDB:**

- Official: Python, Go, Java, Ruby, C
- Bindings auto-generated from core C library
- Consistent API across languages
- Well-documented with examples

**Aspen:**

- Rust only (native)
- Iroh P2P protocol (no HTTP/gRPC)
- No official bindings for other languages
- Limited documentation

**Gap Impact:**

- Cannot be used by non-Rust applications
- Integration with existing systems difficult
- Adoption barrier for teams not using Rust

**Remediation Priority:** MEDIUM

---

## Critical Gaps vs etcd

### 1. **Watch API Guarantees** (SEVERITY: HIGH)

**etcd:**

- Watch on key ranges with prefix matching
- Guaranteed delivery: ordered, reliable, atomic, resumable
- Progress notifications (bookmarks)
- History window for catch-up
- Can watch from specific revision

**Aspen:**

- Log subscription (all entries or by prefix)
- Historical replay from index 0
- Keepalives for connection health
- No revision-based resume
- No progress notifications/bookmarks

**Gap Impact:**

- Harder to implement reliable event-driven architectures
- No guaranteed catch-up after disconnection within window
- Applications must track their own position

**Remediation Priority:** HIGH (important for Kubernetes-like use cases)

### 2. **Lease System Sophistication** (SEVERITY: MEDIUM)

**etcd:**

- First-class lease objects
- Attach multiple keys to one lease
- Keepalive to extend TTL
- Lease grant/revoke through Raft (consistent)
- Lease renewal via leader RPC (efficient)
- Used for: distributed locks, leader election, service discovery

**Aspen:**

- TTL per key or batch
- Lease grant/revoke/keepalive commands exist
- Lazy expiration + 60s background cleanup
- Less battle-tested than etcd

**Gap Impact:**

- Lease precision is lower (60s cleanup interval)
- May not be suitable for time-sensitive use cases
- Service discovery patterns harder to implement

**Remediation Priority:** MEDIUM

### 3. **Compaction and Maintenance** (SEVERITY: MEDIUM)

**etcd:**

- Automatic and manual compaction
- Revision-based compaction (keep N revisions)
- Defragmentation to reclaim space
- Clear maintenance procedures
- Well-documented operational practices

**Aspen:**

- Raft log compaction via snapshots
- SQLite WAL mode for storage
- No explicit compaction API for user data
- No defragmentation story

**Gap Impact:**

- Long-running clusters may have unbounded storage growth
- No way to reclaim space from deleted keys
- Operational overhead unclear

**Remediation Priority:** MEDIUM

### 4. **Production Maturity** (SEVERITY: HIGH)

**etcd:**

- Powers Kubernetes control plane (3300+ contributors)
- CNCF graduated project
- Extensive operational documentation
- Known failure modes and recovery procedures
- Jepsen tested and verified

**Aspen:**

- Experimental/development stage
- Limited production usage
- Documentation focused on developers, not operators
- No third-party verification (Jepsen, etc.)
- Recovery procedures not battle-tested

**Gap Impact:**

- Risk for production deployments
- Unknown failure modes
- Unclear upgrade/migration paths

**Remediation Priority:** HIGH (requires time and community)

---

## Unique Strengths of Aspen

Despite the gaps, Aspen has notable strengths:

1. **Iroh P2P Networking**: NAT traversal, peer discovery, QUIC transport - better for edge/hybrid deployments
2. **madsim Foundation**: Solid deterministic simulation base to build on
3. **Tiger Style**: Explicit resource bounds prevent runaway resource usage
4. **Modern Rust**: Memory safety, async/await, no garbage collection pauses
5. **Capability-based Auth**: Fine-grained delegation chains (8 levels)
6. **Coordination Primitives**: Built-in locks, elections, counters, queues, rate limiters
7. **iroh-docs Integration**: CRDT-based replication for eventually-consistent use cases
8. **Clean Architecture**: Direct async APIs, no actor overhead

---

## Recommended Prioritization

### Phase 1: Foundation (3-6 months)

1. **Enhanced Transaction Model**: Implement OCC with read/write conflict sets
2. **Watch API Improvements**: Add revision-based resume, progress notifications
3. **Compaction/Maintenance**: Add explicit compaction APIs and defragmentation

### Phase 2: Scalability (6-12 months)

4. **Sharding Foundation**: Design multi-shard architecture
5. **Multi-Language Clients**: gRPC or similar for cross-language support
6. **Simulation Enhancements**: Add disk failure simulation, BUGGIFY-style injection

### Phase 3: Ecosystem (12+ months)

7. **Layer Architecture**: Enable building higher-level abstractions
8. **Secondary Indexes**: Either in core or as a layer
9. **Production Hardening**: Third-party testing, operational runbooks

---

## Conclusion

Aspen is a well-architected foundation but needs significant work to compete with FoundationDB (for transaction-heavy workloads) or etcd (for Kubernetes-style coordination). The most critical gaps are:

1. **Transaction model** - prevents building complex applications
2. **Horizontal scaling** - limits deployment size
3. **Watch API** - limits event-driven architectures
4. **Production maturity** - limits enterprise adoption

The good news: Aspen's clean architecture (direct async, Tiger Style, madsim) provides a solid base for addressing these gaps incrementally.

---

## Sources

### FoundationDB

- [Architecture](https://apple.github.io/foundationdb/architecture.html)
- [Features](https://apple.github.io/foundationdb/features.html)
- [Testing](https://apple.github.io/foundationdb/testing.html)
- [Transaction Processing](https://apple.github.io/foundationdb/transaction-processing.html)
- [Record Layer Overview](https://foundationdb.github.io/fdb-record-layer/Overview.html)
- [SIGMOD Paper](https://www.foundationdb.org/files/fdb-paper.pdf)

### etcd

- [API Guarantees](https://etcd.io/docs/v3.5/learning/api_guarantees/)
- [Performance](https://etcd.io/docs/v3.7/op-guide/performance/)
- [Maintenance](https://etcd.io/docs/v3.4/op-guide/maintenance/)
- [GitHub](https://github.com/etcd-io/etcd)
- [Jepsen Analysis](https://jepsen.io/analyses/etcd-3.4.3)

### Comparisons

- [etcd vs FoundationDB](https://stackshare.io/stackups/etcd-vs-foundationdb)
- [db-engines comparison](https://db-engines.com/en/system/FoundationDB;etcd)
- [DST Primer](https://www.amplifypartners.com/blog-posts/a-dst-primer-for-unit-test-maxxers)
