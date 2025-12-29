# Plan: Enhance DocsImporter Client Exposure

## Current State (Already Implemented)

The DocsImporter/PeerManager functionality is **already exposed to clients** via the PeerCluster RPC operations:

### Existing Client RPC Operations (`src/client_rpc.rs:295-343`)

```rust
AddPeerCluster { ticket: String }           // Add peer cluster via AspenDocsTicket
RemovePeerCluster { cluster_id: String }    // Remove peer cluster subscription
ListPeerClusters                            // List all peer clusters
GetPeerClusterStatus { cluster_id: String } // Get sync status for a peer
UpdatePeerClusterFilter { ... }             // Update subscription filter
UpdatePeerClusterPriority { ... }           // Change conflict resolution priority
SetPeerClusterEnabled { ... }               // Enable/disable sync for a peer
```

### Existing Handlers (`src/protocol_handlers/client.rs:1604+`)

All handlers are implemented and call `peer_manager` methods:

- `peer_manager.add_peer(docs_ticket)` - Register new peer cluster
- `peer_manager.remove_peer(&cluster_id)` - Unsubscribe from peer
- `peer_manager.list_peers()` - Get all registered peers
- `peer_manager.get_peer_status(&cluster_id)` - Get sync state
- `peer_manager.update_filter(...)` - Change what keys to sync
- `peer_manager.set_priority(...)` - Change conflict resolution order
- `peer_manager.set_enabled(...)` - Pause/resume sync

### Existing Bootstrap Integration (`src/cluster/bootstrap.rs`)

PeerManager is created during node bootstrap when docs sync is enabled and stored in `NodeHandle.peer_manager: Option<Arc<PeerManager>>`.

---

## Proposed Enhancements

The existing implementation covers peer cluster management. These enhancements would provide deeper visibility and control:

### 1. Add Key Origin Query Operation (P1)

**Purpose**: Allow clients to query which cluster a key originated from.

**New RPC**:

```rust
GetKeyOrigin { key: String } -> Option<KeyOrigin>
```

**Returns**:

```rust
struct KeyOrigin {
    cluster_id: String,
    priority: u32,
    timestamp: u64,
    author_id: String,
}
```

**Implementation**:

- Add `get_key_origin(&self, key: &str) -> Option<KeyOrigin>` to DocsImporter
- Query the SQLite key_origins table
- Add handler in client.rs

### 2. Add Manual Sync Trigger (P2)

**Purpose**: Allow clients to force immediate sync with a peer cluster.

**New RPC**:

```rust
TriggerPeerSync { cluster_id: String } -> SyncResult
```

**Implementation**:

- Add `trigger_sync(&self, cluster_id: &str)` to PeerManager
- Call `docs.sync(namespace, peer).await`
- Return sync stats (entries synced, conflicts resolved)

### 3. Add Sync Progress Subscription (P2)

**Purpose**: Stream real-time sync progress to clients.

**New RPC**:

```rust
SubscribeSyncProgress { cluster_id: Option<String> } -> Stream<SyncProgressEvent>
```

**Events**:

```rust
enum SyncProgressEvent {
    SyncStarted { cluster_id: String },
    EntriesReceived { cluster_id: String, count: u32 },
    ConflictResolved { key: String, winner: String },
    SyncCompleted { cluster_id: String, entries_synced: u32 },
    SyncFailed { cluster_id: String, error: String },
}
```

### 4. Add Docs Namespace Management (P3)

**Purpose**: Direct control over iroh-docs namespaces.

**New RPCs**:

```rust
CreateDocsNamespace { name: String } -> NamespaceId
DeleteDocsNamespace { namespace_id: String } -> bool
ListDocsNamespaces -> Vec<NamespaceInfo>
GetDocsNamespaceTicket { namespace_id: String } -> String
```

### 5. Add Conflict Resolution Stats (P3)

**Purpose**: Visibility into conflict resolution decisions.

**New RPC**:

```rust
GetConflictStats { cluster_id: Option<String>, since: Option<u64> } -> ConflictStats
```

**Returns**:

```rust
struct ConflictStats {
    total_conflicts: u64,
    local_wins: u64,
    remote_wins: u64,
    recent_conflicts: Vec<ConflictEvent>,
}
```

---

## Implementation Order

### Phase 1: Essential Visibility

1. **GetKeyOrigin** - Know where data came from (critical for debugging)

### Phase 2: Operational Control

2. **TriggerPeerSync** - Force sync without waiting for scheduled interval
3. **SubscribeSyncProgress** - Real-time visibility into sync operations

### Phase 3: Advanced Management

4. **Docs Namespace Management** - Direct namespace control
5. **GetConflictStats** - Debugging conflict resolution behavior

---

## File Changes

| File | Changes |
|------|---------|
| `src/client_rpc.rs` | Add 5-8 new request/response variants |
| `src/protocol_handlers/client.rs` | Add handlers for new operations |
| `src/docs/importer.rs` | Add `get_key_origin()` method |
| `src/docs/peer_manager.rs` | Add `trigger_sync()`, progress events |
| `src/docs/store.rs` | Add namespace management methods |

---

## Notes

- The core PeerCluster operations are already complete and functional
- These enhancements add observability and fine-grained control
- Phase 1 is recommended as minimum enhancement
- Phases 2-3 are optional improvements for production deployments
