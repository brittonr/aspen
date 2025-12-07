# ADR-008: Tiger Style Coding Philosophy

**Status:** Accepted

**Date:** 2025-12-03

**Context:** Aspen is a foundational orchestration layer for distributed systems. Such infrastructure requires rigorous engineering discipline to ensure safety, performance, and maintainability. A consistent coding philosophy helps achieve these goals.

## Decision

Adopt **Tiger Style** as Aspen's coding philosophy, emphasizing safety, performance, and developer experience through explicit design principles.

## Philosophy Overview

From `tigerstyle.md`:

> Tiger Style is a coding philosophy focused on safety, performance, and developer experience. Inspired by the practices of TigerBeetle, it focuses on building robust, efficient, and maintainable software through disciplined engineering.

### Three Core Principles

1. **Safety**: Write code that works in all situations and reduces the risk of errors
2. **Performance**: Use resources efficiently to deliver fast, responsive software
3. **Developer Experience**: Create readable and maintainable code that stands the test of time

## Key Practices in Aspen

### 1. Explicit Types and Fixed Limits

**Principle:** Use explicitly sized types (`u64`, `u32`) instead of architecture-dependent types (`usize`).

**Examples from codebase:**

```rust
// src/raft/types.rs
pub type NodeId = u64;  // Not usize - portable across architectures

// src/cluster/gossip_discovery.rs
struct PeerAnnouncement {
    node_id: NodeId,              // u64
    timestamp_micros: u64,        // u64, not usize
    // ...
}

// src/raft/network.rs
const MAX_RPC_MESSAGE_SIZE: usize = 10 * 1024 * 1024;  // 10 MB fixed limit
```

**Rationale:** Prevents size-related bugs when moving between 32-bit and 64-bit architectures. Fixed limits prevent unbounded resource use.

### 2. Fixed Intervals and Bounded Operations

**Principle:** Set explicit upper bounds on loops, queues, and time-based operations.

**Examples:**

```rust
// src/cluster/gossip_discovery.rs
impl GossipPeerDiscovery {
    /// Tiger Style: Fixed interval to prevent unbounded announcement rate
    const ANNOUNCE_INTERVAL_SECS: u64 = 10;

    /// Tiger Style: Bounded wait time (10 seconds max)
    pub async fn shutdown(self) -> Result<()> {
        let timeout = Duration::from_secs(10);
        tokio::select! {
            _ = self.announcer_task => { /* ... */ }
            _ = tokio::time::sleep(timeout) => {
                tracing::warn!("announcer task did not complete within timeout");
            }
        }
    }
}
```

**Rationale:** Prevents infinite loops and runaway resource consumption. Makes system behavior predictable.

### 3. Fail Fast on Programmer Errors

**Principle:** Detect unexpected conditions immediately, stopping faulty code from continuing.

**Examples:**

```rust
// src/cluster/gossip_discovery.rs
let gossip = iroh_manager
    .gossip()
    .context("gossip not enabled on IrohEndpointManager")?;  // Fail immediately

// src/cluster/gossip_discovery.rs
match PeerAnnouncement::from_bytes(&msg.content) {
    Ok(announcement) => { /* process */ }
    Err(e) => {
        tracing::warn!("failed to parse peer announcement: {}", e);
        continue;  // Reject invalid messages immediately
    }
}

// src/raft/storage.rs
async fn purge(&mut self, log_id: LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
    if let Some(prev) = &self.last_purged_log_id {
        assert!(prev <= &log_id);  // Assert invariant
    }
    // ...
}
```

**Rationale:** Bugs are caught immediately rather than propagating through the system.

### 4. Napkin Math for Performance Estimation

**Principle:** Use back-of-the-envelope calculations to estimate system performance.

**Example from gossip discovery:**

> **Network overhead**: 10-node cluster = 10 announcements/sec = ~5 KB/sec (negligible)
>
> **Discovery latency**: New nodes discovered within 10 seconds (acceptable)
>
> **CPU overhead**: Minimal (single broadcast + multiple receives per interval)

**Rationale:** Performance considerations guide design decisions early, avoiding bottlenecks.

### 5. Minimize Variable Scope

**Principle:** Declare variables in the smallest possible scope, close to their usage.

**Examples:**

```rust
// src/cluster/gossip_discovery.rs (good)
pub async fn spawn(...) -> Result<Self> {
    let gossip = iroh_manager.gossip().context("...")?;  // Declared just before use

    let gossip_topic = gossip.subscribe(topic_id, vec![]).await?;
    let (gossip_sender, mut gossip_receiver) = gossip_topic.split();  // Used immediately
    // ...
}
```

**Rationale:** Reduces cognitive load and prevents accidental misuse of variables.

### 6. Clear and Consistent Naming

**Principle:** Use descriptive names with units/qualifiers in descending order of significance.

**Examples:**

```rust
// src/cluster/gossip_discovery.rs
const ANNOUNCE_INTERVAL_SECS: u64 = 10;  // Units included: "SECS"
// Not: ANNOUNCE_INTERVAL or SECS_ANNOUNCE_INTERVAL

// src/simulation.rs
pub duration_ms: u64,  // Units included: "ms"
pub timestamp_micros: u64,  // Units included: "micros"
```

**Pattern:** `{what}_{unit}_{qualifier}` (e.g., `latency_ms_max`, not `max_latency_ms`)

**Rationale:** Avoids confusion about units and groups related variables logically.

## Application in Aspen

### Storage Layer (src/raft/storage.rs)

```rust
// Explicitly sized types for log indices
struct LogStoreInner {
    log: BTreeMap<u64, Entry>,  // u64 indices, not usize
    // ...
}

// Assertions verify invariants
async fn purge(&mut self, log_id: LogIdOf<AppTypeConfig>) -> Result<(), io::Error> {
    if let Some(prev) = &self.last_purged_log_id {
        assert!(prev <= &log_id);  // Fail fast on programmer error
    }
    // ...
}
```

### Network Layer (src/raft/network.rs)

```rust
/// Maximum size for RPC messages (10 MB).
/// Tiger Style: Fixed limit to prevent unbounded memory use.
const MAX_RPC_MESSAGE_SIZE: usize = 10 * 1024 * 1024;
```

### Simulation Testing (src/simulation.rs)

```rust
pub struct SimulationArtifact {
    pub seed: u64,              // u64, not usize
    pub duration_ms: u64,       // Units explicit
    pub timestamp_micros: u64,  // Units explicit
    // ...
}
```

## Alignment with Project Goals

Tiger Style directly supports Aspen's requirements:

1. **Safety**: Distributed consensus requires bulletproof correctness
   - Fixed limits prevent resource exhaustion
   - Assertions catch protocol violations early

2. **Performance**: Orchestration layer must be fast
   - Napkin math guides design decisions
   - Explicit types enable portable optimizations

3. **Developer Experience**: Infrastructure must be maintainable
   - Clear naming reduces cognitive load
   - Consistent style makes code predictable

## Zero Technical Debt

From `tigerstyle.md`:

> **Do it right the first time:** Take the time to design and implement solutions correctly from the start. Rushed features lead to technical debt that requires costly refactoring later.

Aspen's approach:

- Use proven libraries (openraft, redb, iroh) instead of building from scratch
- Write tests alongside implementation
- Document decisions in ADRs
- Refuse shortcuts that create future maintenance burden

## References

- Philosophy document: `tigerstyle.md`
- Original inspiration: TigerBeetle project
- Application throughout: `src/` (examples above)
- Related ADRs: All other ADRs apply Tiger Style principles
