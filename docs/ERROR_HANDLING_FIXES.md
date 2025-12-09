# Aspen Error Handling - Recommended Fixes

This document provides detailed code examples for fixing the critical and moderate issues identified in the error handling audit.

## Critical Issue 1: Serialization Panic in ticket.rs

### Current Code (Line 156)

```rust
pub fn to_bytes(&self) -> Vec<u8> {
    postcard::to_stdvec(&self).expect(
        "AspenClusterTicket postcard serialization failed - \
         indicates library bug or memory corruption"
    )
}
```

### Issue

- Panics in production code when serialization fails (OOM, library bug)
- Called during gossip peer discovery and ticket exchange
- No recovery possible once panic occurs

### Recommended Fix

```rust
#[derive(Debug, thiserror::Error)]
pub enum TicketError {
    #[error("failed to serialize ticket: {0}")]
    Serialize(#[from] postcard::Error),
    #[error("failed to deserialize ticket: {0}")]
    Deserialize(#[from] postcard::Error),
}

impl Ticket for AspenClusterTicket {
    fn to_bytes(&self) -> Result<Vec<u8>, SerializationError> {
        postcard::to_stdvec(&self)
            .map_err(|e| SerializationError::Postcard(e))
    }
}

// Update the trait return type
// impl Ticket for AspenClusterTicket {
//     fn to_bytes(&self) -> Vec<u8> { ... }
// }
// becomes:
// pub fn serialize(&self) -> Result<String, TicketError> {
//     let bytes = self.to_bytes()?;
//     Ok(Self::encode_bytes(&bytes))
// }
```

### Testing

```rust
#[test]
fn test_serialization_error_handling() {
    let ticket = AspenClusterTicket::new(
        TopicId::from_bytes([1u8; 32]),
        "test".into(),
    );
    // Result type allows proper error testing
    let result = ticket.serialize();
    assert!(result.is_ok()); // Normal case
    // Can create test cases for error paths
}
```

---

## Critical Issue 2: Config Validation Panic in aspen-node.rs

### Current Code (Line 516)

```rust
let cluster_client = Arc::new(
    RaftControlClient::new_with_capacity(
        handle.raft_actor.clone(),
        config.raft_mailbox_capacity,
        config.node_id,
    )
    .expect("raft_mailbox_capacity config must be valid (1..=10000)"),
);
```

### Issue

- Panics if configuration validation failed earlier
- Config should be validated before this point, not panicked on
- Operator has no indication of invalid config

### Recommended Fix

**Option A: Validate Config Earlier (Preferred)**

```rust
// In ClusterBootstrapConfig::validate_and_warn()
pub fn validate(&self) -> Result<(), String> {
    if self.raft_mailbox_capacity == 0 || self.raft_mailbox_capacity > 10_000 {
        return Err(format!(
            "invalid raft_mailbox_capacity: {}, must be 1..=10000",
            self.raft_mailbox_capacity
        ));
    }

    if self.election_timeout_min_ms > self.election_timeout_max_ms {
        return Err(format!(
            "election_timeout_min_ms ({}) must be <= election_timeout_max_ms ({})",
            self.election_timeout_min_ms,
            self.election_timeout_max_ms
        ));
    }

    Ok(())
}

// In bootstrap:
let config = load_config(...)?;
config.validate()?;  // Fail fast if invalid
config.validate_and_warn();  // Warn on non-recommended settings

// Later, capacity is guaranteed valid:
let cluster_client = Arc::new(
    RaftControlClient::new_with_capacity(
        handle.raft_actor.clone(),
        config.raft_mailbox_capacity,  // Safe: already validated
        config.node_id,
    )?  // Use ? instead of expect
);
```

**Option B: Handle Error at Creation Point**

```rust
let cluster_client = Arc::new(
    RaftControlClient::new_with_capacity(
        handle.raft_actor.clone(),
        config.raft_mailbox_capacity,
        config.node_id,
    )
    .context("failed to create raft control client with capacity {}",
             config.raft_mailbox_capacity)?
);
```

---

## Critical Issue 3: Serialization Panic in simulation.rs

### Current Code (Lines 245-246)

```rust
let json = serde_json::to_string(&original).expect("serialize");
let deserialized: SimulationArtifact =
    serde_json::from_str(&json).expect("deserialize");
```

### Issue

- Panics in test artifact validation code
- Affects CI/CD reliability
- Should properly handle serialization failures

### Recommended Fix

**Create Helper Function**

```rust
impl SimulationArtifact {
    /// Validate serialization round-trip.
    pub fn validate_serialization(&self) -> Result<()> {
        let json = serde_json::to_string(&self)
            .context("failed to serialize artifact")?;

        let deserialized: SimulationArtifact = serde_json::from_str(&json)
            .context("failed to deserialize artifact")?;

        // Verify round-trip consistency
        if self.run_id != deserialized.run_id {
            return Err(anyhow!(
                "deserialization mismatch: run_id changed"
            ));
        }

        Ok(())
    }
}

// Usage in test code:
#[test]
fn test_artifact_serialization() {
    let artifact = SimulationArtifact::new(
        "test", 42, vec!["event1".into()], "metrics".into()
    );

    artifact.validate_serialization().expect("artifact should round-trip");
}
```

---

## Moderate Issue 1: Silent Reply Send Errors

### Current Code (Lines 193-257 in raft/mod.rs)

```rust
RaftActorMessage::InitCluster(request, reply) => {
    let result = handle_init(state, request).await;
    let _ = reply.send(result);  // Ignores send errors!
}

RaftActorMessage::GetMetrics(reply) => {
    let metrics = state.raft.metrics().borrow().clone();
    let _ = reply.send(Ok(metrics));  // Silent drop
}
```

### Issue

- Reply channel might be closed (caller dropped)
- No indication if client receives response
- Silent failures complicate debugging

### Recommended Fix

**Create Helper Function**

```rust
impl RaftActorState {
    fn send_reply<T: Debug>(
        &self,
        reply: RpcReplyPort<T>,
        result: T,
        context: &str,
    ) {
        if reply.send(result).is_err() {
            warn!(
                node_id = self.node_id,
                context = context,
                "client dropped reply channel"
            );
        }
    }
}

// Usage in message handler:
async fn handle(
    &self,
    myself: ActorRef<Self::Msg>,
    message: Self::Msg,
    state: &mut Self::State,
) -> Result<(), ActorProcessingErr> {
    match message {
        RaftActorMessage::InitCluster(request, reply) => {
            let result = handle_init(state, request).await;
            state.send_reply(reply, result, "InitCluster");
        }
        RaftActorMessage::GetMetrics(reply) => {
            let metrics = state.raft.metrics().borrow().clone();
            state.send_reply(reply, Ok(metrics), "GetMetrics");
        }
        // ... more variants
    }
    Ok(())
}
```

---

## Moderate Issue 2: Silent Configuration Fallback

### Current Code (Lines 302-338 in cluster/config.rs)

```rust
node_id: parse_env("ASPEN_NODE_ID").unwrap_or(0),
http_addr: parse_env("ASPEN_HTTP_ADDR").unwrap_or_else(default_http_addr),
ractor_port: parse_env("ASPEN_RACTOR_PORT").unwrap_or_else(default_ractor_port),
```

### Issue

- Parse errors are silently ignored
- Invalid NODE_ID becomes 0 (valid value, confusing)
- Operators don't know configuration came from defaults

### Recommended Fix

**Create Structured Config Loading**

```rust
fn load_from_env() -> Result<Self> {
    use std::env;
    use tracing::{info, warn};

    // Parse with explicit error handling
    let node_id = match env::var("ASPEN_NODE_ID") {
        Ok(val) => {
            let id: u64 = val.parse()
                .context("failed to parse ASPEN_NODE_ID")?;
            info!(node_id = id, "loaded node_id from environment");
            id
        }
        Err(env::VarError::NotPresent) => {
            info!("ASPEN_NODE_ID not set, using default: 0");
            0
        }
        Err(e) => return Err(e).context("failed to read ASPEN_NODE_ID"),
    };

    let http_addr = match env::var("ASPEN_HTTP_ADDR") {
        Ok(val) => {
            let addr: SocketAddr = val.parse()
                .context("failed to parse ASPEN_HTTP_ADDR")?;
            info!(?addr, "loaded http_addr from environment");
            addr
        }
        Err(env::VarError::NotPresent) => {
            info!("ASPEN_HTTP_ADDR not set, using default: {}", DEFAULT_HTTP_ADDR);
            DEFAULT_HTTP_ADDR
        }
        Err(e) => return Err(e).context("failed to read ASPEN_HTTP_ADDR"),
    };

    // ... more fields

    Ok(Self {
        node_id,
        http_addr,
        // ...
    })
}
```

---

## Missing Feature: Global Panic Hook

### Current Code

No panic hook registered - panics in non-actor contexts might not be logged.

### Recommended Fix

**In src/bin/aspen-node.rs::main()**

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Set up panic hook to log panics
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        tracing::error!(
            panic = ?panic_info,
            "thread panicked"
        );

        // Call default hook to print backtrace
        default_hook(panic_info);
    }));

    // Rest of initialization...
    initialize_logging()?;

    // ... bootstrap code ...

    Ok(())
}
```

This ensures:

- All panics are logged with full context
- Backtraces are still printed to stderr
- Logs can be aggregated by observability system

---

## Missing Feature: Enhanced Error Context in Network Layer

### Current Code (raft/network.rs)

```rust
.map_err(|err| NetworkError::Io(io::Error::other(err.to_string())))?
// Loses error type information
```

### Issue

- Can't distinguish timeout vs connection refused vs corruption
- Complicates debugging and retry logic
- Error details converted to string unnecessarily

### Recommended Fix

**Preserve Error Types**

```rust
pub enum IrpcError {
    #[error("connection timeout after {duration_ms}ms")]
    ConnectTimeout { duration_ms: u64 },

    #[error("connection refused: {reason}")]
    ConnectionRefused { reason: String },

    #[error("stream read timeout after {duration_ms}ms")]
    ReadTimeout { duration_ms: u64 },

    #[error("invalid message format: {reason}")]
    InvalidMessage { reason: String },

    #[error("iroh error: {0}")]
    Iroh(String),
}

impl From<IrpcError> for openraft::network::NetworkError {
    fn from(err: IrpcError) -> Self {
        match err {
            IrpcError::ConnectTimeout { .. } => {
                NetworkError::Timeout("connection timeout")
            }
            IrpcError::ReadTimeout { .. } => {
                NetworkError::Timeout("read timeout")
            }
            _ => NetworkError::Io(io::Error::other(err.to_string())),
        }
    }
}

// Usage:
async fn send_request(&self, request: RaftRequest) -> Result<Response, IrpcError> {
    let response = tokio::time::timeout(
        Duration::from_millis(self.timeout_ms),
        self.inner_send(request)
    )
    .await
    .map_err(|_| IrpcError::ReadTimeout {
        duration_ms: self.timeout_ms
    })?;

    Ok(response)
}
```

---

## Summary of Changes

| Priority | File | Change | Effort | Impact |
|----------|------|--------|--------|--------|
| 1 | ticket.rs | Return Result instead of panicking | Medium | High |
| 1 | aspen-node.rs | Use ? operator, validate config earlier | Low | High |
| 1 | simulation.rs | Use context() and ? operators | Low | Medium |
| 2 | raft/mod.rs | Log reply send failures | Low | Medium |
| 2 | cluster/mod.rs | Log cast failures | Low | Low |
| 2 | cluster/config.rs | Log when using defaults | Low | Medium |
| 3 | aspen-node.rs | Add panic hook | Low | Medium |
| 3 | raft/network.rs | Preserve error types | Medium | Medium |

---

## Testing Recommendations

1. **Add unit tests for error paths**
   - Test serialization failures
   - Test config validation failures
   - Test reply send failures

2. **Add integration tests**
   - Test actor recovery after simulated panics
   - Test network timeouts and retries
   - Test config validation in full bootstrap

3. **Add chaos testing**
   - Simulate serialization failures
   - Simulate network partitions
   - Simulate actor crashes

4. **Performance testing**
   - Ensure error handling doesn't regress performance
   - Test high-frequency error logging
   - Test memory usage of error types

---

## Rollout Plan

1. **Phase 1 (Immediate)**: Fix critical panics (Issues 1-3)
   - Minimal API changes
   - High safety improvement

2. **Phase 2 (Short-term)**: Add logging for moderate issues (Issues 4-6)
   - No API changes
   - Better observability

3. **Phase 3 (Medium-term)**: Add panic hook and improve network errors
   - Operational improvement
   - Better debugging

4. **Phase 4 (Ongoing)**: Comprehensive test coverage for error paths
   - Ensures no regressions
   - Documents expected behavior
