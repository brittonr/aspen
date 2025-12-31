# Plan: Breaking the raft ↔ cluster Circular Dependency

## Status: COMPLETED

This refactoring has been successfully implemented. See below for details.

## Problem Summary

The `raft` and `cluster` modules had a circular dependency:

**raft → cluster:**
- `connection_pool.rs:72`: `use crate::cluster::IrohEndpointManager;`
- `network.rs:89`: `use crate::cluster::IrohEndpointManager;`
- `server.rs:44`: `use crate::cluster::IrohEndpointManager;`

**cluster → raft:**
- `transport.rs:170`: `use crate::raft::types::NodeId;`
- `bootstrap.rs`: 16 imports from raft
- `gossip_discovery.rs`: 13 imports from raft
- `config.rs`: 1 import from raft

## Solution Architecture

The recent commit efc6435 introduced `NetworkTransport` and `PeerDiscovery` traits. To complete the modularization:

### Phase 1: Move NodeId to Neutral Location

Move `NodeId` from `raft/types.rs` to `api/mod.rs`. This allows both `raft` and `cluster` to import it without circular deps.

**Changes:**
1. Add `NodeId` definition to `src/api/mod.rs`
2. Re-export from `raft/types.rs` for backward compatibility
3. Update `cluster/transport.rs` to import from `api`

### Phase 2: Make raft Generic Over NetworkTransport

Instead of importing `IrohEndpointManager` directly, make the raft networking components accept a generic `T: NetworkTransport`.

**Changes:**
1. `RaftConnectionPool<T: NetworkTransport>` - stores `Arc<T>` instead of `Arc<IrohEndpointManager>`
2. `IrpcRaftNetworkFactory<T: NetworkTransport>` - generic over transport
3. `RaftRpcServer::spawn<T: NetworkTransport>()` - accept trait object

### Phase 3: Implement PeerDiscovery for GossipPeerDiscovery

The `PeerDiscovery` trait is defined but not implemented by `GossipPeerDiscovery`.

**Changes:**
1. Refactor `GossipPeerDiscovery::spawn()` to `start()` with callback
2. Implement `announce()` method
3. Implement `is_running()` method
4. Update callers in `bootstrap.rs`

## Dependency Graph After Changes

```
                    ┌─────────────────┐
                    │      api/       │
                    │   (NodeId,      │
                    │    Traits)      │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
       ┌──────────┐   ┌──────────┐   ┌──────────┐
       │   raft/  │   │ cluster/ │   │   node/  │
       │(generic) │   │ (impls)  │   │(builder) │
       └──────────┘   └──────────┘   └──────────┘
              ▲              │
              │              │
              └──────────────┘
           (cluster implements
            transport traits)
```

## Implementation Steps

### Step 1: Move NodeId to api (Low Risk)
- Add NodeId struct to `src/api/mod.rs`
- Keep re-export in `raft/types.rs` for backward compat
- Update `cluster/transport.rs` import

### Step 2: Extract Transport Constants
- Move transport-related constants to a shared location
- Constants like `IROH_CONNECT_TIMEOUT`, `MAX_STREAMS_PER_CONNECTION`

### Step 3: Make RaftConnectionPool Generic
- Change `endpoint_manager: Arc<IrohEndpointManager>` to `transport: Arc<T>`
- Use `T::Endpoint` for connection operations
- This requires adding connection methods to `NetworkTransport` trait

### Step 4: Make IrpcRaftNetworkFactory Generic
- Similar to RaftConnectionPool
- The factory creates `IrpcRaftNetwork` instances for each peer

### Step 5: Update RaftRpcServer
- Accept `&dyn NetworkTransport` or generic parameter
- Use trait methods for accepting connections

### Step 6: Implement PeerDiscovery for GossipPeerDiscovery
- Add `impl PeerDiscovery for GossipPeerDiscovery`
- Refactor `spawn()` to use callback pattern
- Add `announce()` and `is_running()` methods

### Step 7: Update Bootstrap
- Use trait-based discovery
- Pass callbacks instead of direct network factory access

## Testing Strategy

1. Run `cargo check` after each step to catch compile errors
2. Run quick tests: `cargo nextest run -P quick`
3. Run full test suite before merging

## Rollback Plan

Each step is independently revertible. If issues arise:
1. Revert the specific commit
2. The previous code will work unchanged

## Estimated Complexity

- Phase 1 (NodeId move): Low - pure refactoring, no logic changes
- Phase 2 (Generics): Medium - requires careful type threading
- Phase 3 (PeerDiscovery impl): Medium - callback refactoring

Total: ~400-600 lines of changes across ~10 files

---

## Completed Work Summary

The following changes were implemented to break the circular dependency:

### 1. Moved NodeId to api module (DONE)
- Added `NodeId` struct definition to `src/api/mod.rs`
- Re-exported from `raft/types.rs` for backward compatibility
- Updated `cluster/transport.rs` to import from `api` instead of `raft`

### 2. Moved NetworkTransport trait to api module (DONE)
- Created `src/api/transport.rs` with all transport traits and types
- `NetworkTransport`, `IrohTransportExt`, `PeerDiscovery` traits now in `api`
- `DiscoveredPeer`, `DiscoveryHandle`, `PeerDiscoveredCallback` types in `api`
- `cluster/transport.rs` now re-exports from `api::transport` for backward compat

### 3. Made raft networking generic over NetworkTransport (DONE)
- `RaftConnectionPool<T>` is now generic over `T: NetworkTransport`
- `IrpcRaftNetworkFactory<T>` is now generic over `T: NetworkTransport`
- `IrpcRaftNetwork<T>` is now generic over `T: NetworkTransport`
- Manual `Clone` implementations (avoids T: Clone bound from derive)

### 4. Type aliases in cluster module (DONE)
- `cluster::IrpcRaftNetworkFactory` = `IrpcRaftNetworkFactory<IrohEndpointManager>`
- `cluster::RaftConnectionPool` = `RaftConnectionPool<IrohEndpointManager>`
- `cluster::IrpcRaftNetwork` = `IrpcRaftNetwork<IrohEndpointManager>`

### Dependency Graph After Changes

```
                    ┌─────────────────┐
                    │      api/       │
                    │  (NodeId,       │
                    │   NetworkTransport,│
                    │   PeerDiscovery)│
                    └────────┬────────┘
                             │ (both import from api)
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
       ┌──────────┐   ┌──────────┐   ┌──────────┐
       │   raft/  │   │ cluster/ │   │   node/  │
       │(generic T)│   │(concrete │   │(builder) │
       └──────────┘   │  impls)  │   └──────────┘
                      └──────────┘
```

- **raft** module: Generic types with `T: NetworkTransport` bounds
- **cluster** module: Provides `IrohEndpointManager` (impl) + type aliases
- **api** module: Shared types and traits (neutral ground)
- **No direct imports from raft → cluster** for transport types
