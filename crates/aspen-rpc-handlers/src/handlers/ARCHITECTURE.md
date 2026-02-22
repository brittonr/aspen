# Handler Architecture Specification

This document describes the handler architecture for Aspen's RPC layer, including
patterns for core infrastructure handlers and guidelines for extracting
application-layer handlers to separate crates.

## Overview

Aspen handlers are divided into two categories:

1. **Core Infrastructure Handlers** - Fundamental cluster operations, kept
   inline in `aspen-rpc-handlers` for simplicity
2. **Application-Layer Handlers** - Domain-specific features extracted to
   separate crates for modularity and faster incremental builds

## Architecture Diagram

```
                      aspen-rpc-core
                           |
                           | (RequestHandler trait,
                           |  ClientProtocolContext)
                           |
         +-----------------+------------------+
         |                                    |
         v                                    v
  aspen-rpc-handlers               aspen-*-handler crates
  (core handlers inline)           (application handlers)
         |                                    |
         |  - CoreHandler                     |  - BlobHandler
         |  - KvHandler                       |  - CoordinationHandler
         |  - ClusterHandler                  |  - ForgeHandler
         |  - LeaseHandler                    |  - CiHandler
         |  - WatchHandler                    |  - SecretsHandler
         |  - ServiceRegistryHandler          |  - (future extractions)
         |  - DocsHandler                     |
         |                                    |
         +----------------+-------------------+
                          |
                          v
                   HandlerRegistry
                   (dispatches to all)
```

## Classification Criteria

### Keep Inline When

- Core cluster functionality required for basic operation
- No external feature dependencies or always enabled
- Small, stable implementation (< 300 lines)
- Shared by many other features
- No dedicated domain crate

### Extract to Separate Crate When

- Application-specific domain logic (git, CI, secrets, DNS, SQL)
- Feature-gated with `#[cfg(feature = "...")]`
- Large implementation with domain-specific dependencies
- Has a dedicated domain crate (e.g., `aspen-forge`, `aspen-ci`)
- Would benefit from independent testing or faster incremental builds

## RequestHandler Trait

All handlers implement the `RequestHandler` trait from `aspen-rpc-core`:

```rust
use anyhow::Result;
use aspen_client_api::{ClientRpcRequest, ClientRpcResponse};
use async_trait::async_trait;
use crate::ClientProtocolContext;

#[async_trait]
pub trait RequestHandler: Send + Sync {
    /// Returns true if this handler can process the given request.
    /// Must be fast and not perform any I/O.
    fn can_handle(&self, request: &ClientRpcRequest) -> bool;

    /// Process the request and return a response.
    /// Errors are sanitized before being sent to the client.
    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> Result<ClientRpcResponse>;

    /// Returns the handler name for logging/debugging.
    fn name(&self) -> &'static str;
}
```

## Core Infrastructure Handlers (Inline)

These handlers remain in `aspen-rpc-handlers/src/handlers/`:

| Handler | Operations | Feature | Notes |
| ------- | ---------- | ------- | ----- |
| `CoreHandler` | Ping, Health, Metrics, NodeInfo, Leader | always | Cluster health and observability |
| `KvHandler` | Read, Write, Delete, Scan, CAS, Batch | always | Distributed key-value operations |
| `ClusterHandler` | Init, AddLearner, Membership, Tickets | always | Raft cluster management |
| `LeaseHandler` | AcquireLease, RenewLease, ReleaseLease | always | Distributed leases |
| `WatchHandler` | Watch, Unwatch | always | Key change notifications |
| `ServiceRegistryHandler` | Register, Discover, Heartbeat | always | Service discovery |
| `DocsHandler` | DocsSet, DocsGet, DocsSync | always | iroh-docs CRDT operations |

## Application-Layer Handler Crates

### Already Extracted

| Crate | Handler | Domain | Feature Flag |
| ----- | ------- | ------ | ------------ |
| `aspen-blob-handler` | `BlobHandler` | Content-addressed storage | `blob` |
| `aspen-coordination-handler` | `CoordinationHandler` | Locks, counters, queues, barriers | (always) |
| `aspen-forge-handler` | `ForgeHandler` | Decentralized Git hosting | `forge` |
| `aspen-ci-handler` | `CiHandler` | CI/CD pipelines | `ci` |
| `aspen-secrets-handler` | `SecretsHandler` | Vault-compatible secrets | `secrets` |

### To Be Extracted

| Current File | Target Crate | Domain | Feature Flag | Priority |
| ------------ | ------------ | ------ | ------------ | -------- |
| `snix.rs` | `aspen-nix-handler` | Nix store operations | `snix` | 1 |
| `dns.rs` | `aspen-query-handler` | DNS records | `dns` | 2 |
| `sql.rs` | `aspen-query-handler` | SQL queries | `sql` | 3 |
| `hooks.rs` | `aspen-hooks-handler` | Event automation | (always) | 4 |
| `job.rs` + `worker.rs` | `aspen-job-handler` | Job queue, worker coordination | (always) | 5 |
| ~~automerge~~ | `aspen-automerge-plugin` (WASM) | CRDT sync | `automerge` | 6 |
| `cache.rs` + `cache_migration.rs` | `aspen-cache-handler` | CI build cache | `ci` | 7 |
| `pijul.rs` | `aspen-pijul-handler` | Pijul VCS | `pijul` | 8 |

## Creating a New Application-Layer Handler Crate

### Crate Structure

```
aspen-<domain>-handler/
├── Cargo.toml
└── src/
    ├── lib.rs       # Re-exports (15-25 lines)
    └── handler.rs   # Implementation
```

### Step 1: Create Crate Directory

```bash
mkdir -p crates/aspen-<domain>-handler/src
```

### Step 2: Cargo.toml

```toml
[package]
name = "aspen-<domain>-handler"
version = "0.1.0"
edition = "2024"
description = "<Domain> RPC handler for Aspen distributed cluster"
license = "AGPL-3.0-or-later"

[dependencies]
# RPC core abstractions (RequestHandler trait, ClientProtocolContext)
aspen-rpc-core = { path = "../aspen-rpc-core" }

# Domain crate with business logic
aspen-<domain> = { path = "../aspen-<domain>" }

# Client API types
aspen-client-api = { workspace = true }

# Core dependencies
async-trait = "0.1"
anyhow = "1.0"
tracing = "0.1"

[features]
default = []
# Add feature flags matching aspen-rpc-core if needed

[dev-dependencies]
tokio = { version = "1.0", features = ["full", "test-util"] }
```

### Step 3: lib.rs

```rust
//! <Domain> RPC handler for Aspen.
//!
//! This crate provides the <domain> handler extracted from
//! aspen-rpc-handlers for better modularity and faster incremental builds.
//!
//! Handles <domain> operations:
//! - Operation1: Description
//! - Operation2: Description

mod handler;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::RequestHandler;
pub use handler::<Domain>Handler;
```

### Step 4: handler.rs

```rust
//! <Domain> request handler.
//!
//! Handles: Operation1, Operation2, ...

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::{Operation1Response, Operation2Response};
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;
use tracing::warn;

/// Sanitize domain-specific errors for client consumption.
///
/// Domain errors can contain internal details (file paths, connection strings).
/// Categorize them into user-safe messages.
fn sanitize_<domain>_error(err: &aspen_<domain>::<Domain>Error) -> String {
    use aspen_<domain>::<Domain>Error;
    match err {
        <Domain>Error::NotFound { .. } => "<domain> not found".to_string(),
        <Domain>Error::InvalidInput { .. } => "invalid input".to_string(),
        <Domain>Error::Internal { .. } => "internal error".to_string(),
    }
}

/// Handler for <domain> operations.
pub struct <Domain>Handler;

#[async_trait::async_trait]
impl RequestHandler for <Domain>Handler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        matches!(
            request,
            ClientRpcRequest::Operation1 { .. }
                | ClientRpcRequest::Operation2 { .. }
        )
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::Operation1 { arg1, arg2 } => {
                handle_operation1(ctx, arg1, arg2).await
            }
            ClientRpcRequest::Operation2 { arg } => {
                handle_operation2(ctx, arg).await
            }
            _ => Err(anyhow::anyhow!("request not handled by <Domain>Handler")),
        }
    }

    fn name(&self) -> &'static str {
        "<Domain>Handler"
    }
}

// ============================================================================
// Operation Handlers
// ============================================================================

async fn handle_operation1(
    ctx: &ClientProtocolContext,
    arg1: String,
    arg2: u64,
) -> anyhow::Result<ClientRpcResponse> {
    // Check if required service is available
    let Some(ref service) = ctx.<domain>_service else {
        return Ok(ClientRpcResponse::Operation1Result(Operation1Response {
            success: false,
            error: Some("<domain> service not enabled".to_string()),
        }));
    };

    match service.operation1(&arg1, arg2).await {
        Ok(result) => {
            Ok(ClientRpcResponse::Operation1Result(Operation1Response {
                success: true,
                error: None,
            }))
        }
        Err(e) => {
            warn!(error = %e, "operation1 failed");
            Ok(ClientRpcResponse::Operation1Result(Operation1Response {
                success: false,
                error: Some(sanitize_<domain>_error(&e)),
            }))
        }
    }
}

async fn handle_operation2(
    ctx: &ClientProtocolContext,
    arg: String,
) -> anyhow::Result<ClientRpcResponse> {
    // Implementation...
    todo!()
}
```

### Step 5: Add to Workspace

```toml
# Cargo.toml (workspace root)
[workspace]
members = [
    # ... existing members
    "crates/aspen-<domain>-handler",
]
```

### Step 6: Feature Flag in aspen-rpc-handlers

```toml
# crates/aspen-rpc-handlers/Cargo.toml
[features]
<domain> = ["dep:aspen-<domain>-handler", "aspen-rpc-core/<domain>"]

[dependencies]
aspen-<domain>-handler = { path = "../aspen-<domain>-handler", optional = true }
```

### Step 7: Update mod.rs

```rust
// crates/aspen-rpc-handlers/src/handlers/mod.rs

// Remove the inline module
// pub mod <domain>;  // DELETE THIS

// Add re-export
#[cfg(feature = "<domain>")]
pub use aspen_<domain>_handler::<Domain>Handler;
```

### Step 8: Update registry.rs

```rust
// crates/aspen-rpc-handlers/src/registry.rs

// Add handler registration
#[cfg(feature = "<domain>")]
if ctx.<domain>_service.is_some() {
    handlers.push(Arc::new(<Domain>Handler));
}
```

## Migration Checklist

For each inline handler to extract:

- [ ] Create `aspen-<domain>-handler` crate directory
- [ ] Write `Cargo.toml` with correct dependencies
- [ ] Move implementation from `handlers/<name>.rs` to `handler.rs`
- [ ] Create `lib.rs` with re-exports
- [ ] Update imports to use `aspen_rpc_core` instead of `crate`
- [ ] Add to workspace `Cargo.toml`
- [ ] Add feature flag in `aspen-rpc-handlers/Cargo.toml`
- [ ] Update `handlers/mod.rs` to re-export from handler crate
- [ ] Update `registry.rs` handler registration
- [ ] Move/update tests (if any)
- [ ] Delete original `handlers/<name>.rs`
- [ ] Run `cargo build && cargo nextest run`
- [ ] Update this document's "Already Extracted" table

## Error Handling

### Error Sanitization

Every handler crate should include a `sanitize_*_error` function that converts
internal errors to user-safe messages. This prevents leaking:

- Internal file paths
- Connection strings or credentials
- Stack traces
- Implementation details

Example pattern:

```rust
fn sanitize_blob_error(err: &aspen_blob::BlobStoreError) -> String {
    use aspen_blob::BlobStoreError;
    match err {
        BlobStoreError::NotFound { .. } => "blob not found".to_string(),
        BlobStoreError::TooLarge { max, .. } => format!("blob too large; max {} bytes", max),
        BlobStoreError::Storage { .. } => "storage error".to_string(),
        BlobStoreError::Download { .. } => "download failed".to_string(),
        BlobStoreError::InvalidTicket { .. } => "invalid ticket".to_string(),
    }
}
```

### Response Patterns

For operations that can fail gracefully:

```rust
// Return success/error in response struct (preferred)
Ok(ClientRpcResponse::OperationResult(OperationResponse {
    success: false,
    error: Some("service not enabled".to_string()),
}))
```

For unexpected failures:

```rust
// Return anyhow error (will be sanitized by registry)
Err(anyhow::anyhow!("request not handled by <Domain>Handler"))
```

## Testing

### Unit Tests for can_handle()

Test that the handler correctly identifies which requests it handles:

```rust
#[test]
fn test_can_handle_operations() {
    let handler = <Domain>Handler;
    assert!(handler.can_handle(&ClientRpcRequest::Operation1 { .. }));
    assert!(handler.can_handle(&ClientRpcRequest::Operation2 { .. }));
}

#[test]
fn test_rejects_unrelated_requests() {
    let handler = <Domain>Handler;
    assert!(!handler.can_handle(&ClientRpcRequest::Ping));
    assert!(!handler.can_handle(&ClientRpcRequest::ReadKey { .. }));
}
```

### Integration Tests

Use `TestContextBuilder` from `aspen_rpc_core::test_support`:

```rust
use aspen_rpc_core::test_support::TestContextBuilder;

async fn setup_test_context() -> ClientProtocolContext {
    TestContextBuilder::new()
        .with_node_id(1)
        .with_endpoint_manager(mock_endpoint)
        .build()
}

#[tokio::test]
async fn test_handle_operation1() {
    let ctx = setup_test_context().await;
    let handler = <Domain>Handler;

    let request = ClientRpcRequest::Operation1 { arg: "test".into() };
    let result = handler.handle(request, &ctx).await;

    assert!(result.is_ok());
    // Verify response...
}
```

## Tiger Style Checklist

Handlers must follow Tiger Style principles:

- [ ] **Stateless**: Handlers hold no state; all state comes from `ClientProtocolContext`
- [ ] **No `.unwrap()`/`.expect()`**: Use `?` with context or return error responses
- [ ] **No `panic!()`/`todo!()`**: Return appropriate error responses
- [ ] **70-line limit**: Keep individual functions under 70 lines
- [ ] **No locks across `.await`**: Never hold mutex guards across await points
- [ ] **Bounded operations**: Cap limits (e.g., `limit.min(1000)`)
- [ ] **Error sanitization**: Never leak internal details to clients
- [ ] **Units in names**: `timeout_ms`, `size_bytes`, `duration_us`
