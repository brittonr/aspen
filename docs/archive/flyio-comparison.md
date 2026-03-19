# Aspen vs Fly.io Stack: Corrosion, LiteFS, Sprites

## References

- [Corrosion blog post](https://fly.io/blog/corrosion/) — war stories and architecture
- [Corrosion source](https://github.com/superfly/corrosion) — gossip-based service discovery
- [LiteFS docs](https://fly.io/docs/litefs/) — distributed SQLite via FUSE
- [LiteFS how it works](https://fly.io/docs/litefs/how-it-works/) — page-level replication internals
- [Sprites](https://sprites.dev) — stateful Firecracker sandboxes with checkpoint/restore

---

## Context

Fly.io runs a global platform of Firecracker micro-VMs. They've built several distributed systems components to solve their specific problems: routing state propagation, edge-local databases, and sandboxed compute. Each one targets a narrow slice of what Aspen provides as a unified system.

This comparison examines where the approaches overlap, where they diverge, and what Aspen can learn from Fly.io's hard-won operational experience.

---

## System-by-System Comparison

### Corrosion vs Aspen

Corrosion propagates a SQLite database across thousands of nodes using gossip + CRDTs. It explicitly rejects distributed consensus.

| Dimension | Corrosion | Aspen |
|-----------|-----------|-------|
| **Consistency** | Eventual (cr-sqlite CRDTs, LWW) | Linearizable (Raft consensus) |
| **Transport** | QUIC (Quinn) | QUIC (Iroh, with NAT traversal + discovery) |
| **Membership** | SWIM gossip (Foca) | Gossip + mDNS + DNS + DHT |
| **Storage** | SQLite per node | Redb (unified log + state machine) |
| **Conflict resolution** | cr-sqlite CRDTs (last-write-wins) | CAS operations via Raft (no conflicts) |
| **Query** | SQL (SQLite) | KV + optional DataFusion SQL |
| **Schema** | File-based, hot-reloaded | Programmatic (Rust types) |
| **Scale target** | Thousands of nodes (Fly.io fleet) | Smaller clusters with strong consistency |
| **Language** | Rust | Rust |

**Why Fly.io chose eventual consistency:** Their workers own their own state. A worker in Tokyo knows what Fly Machines it runs — that's not something São Paulo needs to vote on. Updates from different workers almost never conflict, making consensus overhead wasteful. Corrosion exploits this by using CRDTs where LWW is good enough.

**Why Aspen chose Raft:** Aspen's primitives — distributed locks, leader elections, atomic counters, queues — require linearizability. You can't build a correct fencing token with eventual consistency. Different problem domain, different tradeoff.

**Where they meet:** Both use QUIC for transport. Both are written in Rust. Both gossip for cluster membership. The networking stack is similar in spirit, though Aspen's Iroh layer adds NAT traversal and content-addressed blob transfer that Corrosion doesn't need.

#### Operational lessons from Corrosion

Fly.io's blog post is unusually candid about failures. Three patterns are directly relevant to Aspen:

1. **Watchdogs for async runtimes.** Corrosion hit a contagious deadlock from `if let` over an `RwLock` where the else branch held the lock. Their fix: watchdogs on every Tokio program that detect event-loop stalls and bounce the service. Aspen's Tiger Style rule "never hold locks across `.await` points" prevents the root cause, but a watchdog catches the bugs that slip through anyway.

2. **Schema migrations as distributed events.** Adding a nullable column to a CRDT table caused cr-sqlite to backfill every row, which propagated as if every Fly Machine changed state simultaneously. Aspen's redb schema changes don't have this problem (no CRDT backfill), but any operation that touches all keys in a prefix could trigger similar amplification during Raft log replay.

3. **Regionalization reduces blast radius.** Corrosion started with a single global cluster and moved to two-level: per-region clusters for fine-grained state, global cluster for app-to-region mapping. Aspen's federation design (`docs/FEDERATION.md`) already plans for multi-cluster, but the Corrosion experience validates the approach — single global state is a liability at scale.

4. **Full-state republishing beats incremental updates.** Corrosion moved from incremental updates to republishing entire Fly Machine state on change. The CRDT layer filters no-op changes before gossiping. Simpler, fewer bugs. Aspen's Raft state machine applies complete writes, which is the same pattern.

---

### LiteFS vs Aspen

LiteFS transparently replicates SQLite via a FUSE filesystem. Applications read/write SQLite normally; LiteFS intercepts filesystem calls, captures page-level diffs (LTX format), and ships them to replicas.

| Dimension | LiteFS | Aspen |
|-----------|--------|-------|
| **Consistency** | Async replication (single-writer) | Linearizable (Raft majority writes) |
| **Replication unit** | SQLite page sets (LTX files) | Raft log entries → redb state machine |
| **Write model** | Single primary, lease-based failover | Raft leader election |
| **Read model** | Local reads on all nodes | Leader reads or follower reads (configurable) |
| **Split brain** | Detected via rolling checksum, snapshot recovery | Prevented by Raft majority quorum |
| **Primary election** | Consul lease | Raft term-based election |
| **Interface** | FUSE mount (transparent to apps) | Rust trait API (`KeyValueStore`) |
| **Transport** | HTTP | QUIC (Iroh) |
| **Durability** | Async (writes may be lost on primary failure) | Synchronous (majority commit) |
| **Status** | Unsupported, pre-1.0, data loss risk with autoscaler | Production-ready |

**Key architectural difference:** LiteFS trades durability for write throughput — the primary acknowledges writes before replicas confirm. A primary crash can lose recent transactions. The rolling checksum detects this (split brain) and snapshots from the new primary, but those writes are gone.

Aspen's Raft requires majority acknowledgment before a write is committed. Slower, but no data loss on leader failure. The single-fsync redb architecture (1.65ms writes) keeps the latency penalty small.

**What LiteFS gets right:** The FUSE approach is clever — applications don't need to know about replication. Any SQLite app gets distributed reads for free. Aspen's `aspen-fuse` crate does something similar for the KV store, but the primary use case is different (filesystem interface to distributed state vs. transparent database replication).

**⚠️ LiteFS is effectively abandoned.** Fly.io's docs warn: "We are not able to provide support or guidance for this product." They explicitly warn against combining it with their own autoscaler due to data loss risk. The lesson: transparent replication via filesystem interception is fragile when the orchestrator doesn't understand replication state.

---

### Sprites vs Aspen Jobs/CI

Sprites are stateful Firecracker VMs with checkpoint/restore, persistent ext4 filesystems, and per-use billing. They're positioned for running AI agents and untrusted code.

| Dimension | Sprites | Aspen Jobs |
|-----------|---------|------------|
| **Isolation** | Firecracker VM (hardware isolation) | VM (Hyperlight) or shell workers |
| **Persistence** | ext4 on NVMe, object storage backup | Job artifacts in iroh-blobs |
| **Checkpointing** | VM-level snapshot (~300ms) | Job state in Raft KV |
| **Scheduling** | On-demand, auto-hibernate | Raft-coordinated job queue |
| **Networking** | HTTP proxy per sprite, egress policies | Iroh P2P per worker |
| **Cost model** | Pay-per-use (CPU-seconds, GB-hours) | Self-hosted |
| **Build system** | None (general purpose) | Nix builds via snix |

**Different goals:** Sprites is a managed service for running arbitrary code blobs. Aspen's job system is infrastructure you run yourself for CI/CD pipelines and distributed task execution.

**What Sprites demonstrates:** The market for sandboxed compute with persistent state is real. Sprites' checkpoint/restore and auto-hibernate model (stop billing when idle, resume with same filesystem) maps roughly to what Aspen's VM executor could offer — submit a job, it runs in an isolated VM, artifacts persist in the blob store, the VM goes away.

**Relevant to Aspen CI:** Sprites charges $0.44 for a 4-hour Claude Code session. That's the pricing pressure for self-hosted alternatives. Aspen CI running nix builds in Hyperlight VMs avoids that per-use cost entirely, which matters for organizations running hundreds of builds per day.

---

## Unified Comparison Table

| Feature | Corrosion | LiteFS | Sprites | Aspen |
|---------|-----------|--------|---------|-------|
| **Consistency** | Eventual (CRDT) | Async replication | N/A | Linearizable (Raft) |
| **Storage** | SQLite | SQLite (FUSE) | ext4 | Redb |
| **Transport** | QUIC (Quinn) | HTTP | HTTPS proxy | QUIC (Iroh) |
| **Conflict resolution** | Last-write-wins | Single-writer | N/A | CAS via Raft |
| **Coordination primitives** | None | Consul lease only | None | Locks, elections, queues, barriers |
| **Formal verification** | None | None | None | 27 Verus modules |
| **Simulation testing** | Antithesis (external) | None | None | madsim (built-in) |
| **Self-hosting** | No | No | No | Yes (Forge + CI + Nix) |
| **Status** | Production (Fly.io internal) | Unsupported | GA | Production-ready |

---

## What Aspen Should Take From This

### Already aligned

- **QUIC transport**: Both Corrosion and Aspen chose QUIC. Correct call.
- **Gossip for membership**: SWIM variants in both. Aspen adds multiple discovery mechanisms.
- **Rust**: All three Fly.io systems and Aspen are Rust. The ecosystem works for distributed systems.
- **Federation/regionalization**: Corrosion learned the hard way that single global state is fragile. Aspen's federation design already accounts for this.

### Worth adopting

1. **Tokio watchdogs.** Corrosion's contagious deadlock bug is a class of failure Aspen hasn't explicitly defended against. A simple watchdog that detects event-loop stalls and restarts the service is cheap insurance. Tiger Style prevents most lock-across-await bugs, but defense in depth matters.

2. **Antithesis-style testing.** Corrosion used Antithesis for multiverse debugging and found real bugs (including the `parking_lot` issue). Aspen uses madsim for deterministic simulation, which catches different classes of bugs. The two approaches are complementary — madsim for reproducible distributed scenarios, Antithesis/deterministic hypervisors for concurrency bugs in the runtime itself.

3. **Schema migration as blast event.** Any operation that touches O(n) keys atomically is a potential amplification vector. Aspen should document this pattern and ensure bulk operations (prefix deletes, migrations) have rate limiting or batching built in.

### Validates Aspen's choices

- **Raft over CRDTs for coordination.** Corrosion explicitly cannot do distributed locks, leader elections, or atomic counters. Fly.io uses separate systems (tkdb with Litestream, Consul leases) for state that needs stronger guarantees. Aspen provides all of this in one system.

- **Single unified system over point solutions.** Fly.io runs Corrosion + LiteFS + Consul + tkdb + Litestream + Pet Sematary for different state management needs. Each has its own failure modes and operational burden. Aspen unifies KV, coordination, blob storage, and consensus into one deployment.

- **Self-hosted over managed.** Sprites is $0.44 per 4-hour session. At scale, that adds up. Aspen CI on your own hardware avoids the per-use tax entirely.

---

## Key Takeaway

Fly.io's stack is a collection of purpose-built tools, each optimized for one narrow problem. Corrosion is great at propagating routing state across thousands of nodes without consensus overhead. LiteFS was a clever hack for edge-local SQLite. Sprites packages Firecracker VMs as a managed service.

Aspen is a unified platform. It trades some of Corrosion's scale (thousands of nodes) for consistency guarantees that Corrosion can't provide. It avoids LiteFS's durability gap by using Raft majority commits. It replaces Sprites' managed model with self-hosted infrastructure.

The systems aren't competitors — they solve different problems at different layers. But for organizations that need distributed coordination primitives (not just state propagation), Aspen provides what Fly.io's stack cannot: linearizable operations, formal verification, and a self-hosting path that eliminates vendor dependency.
