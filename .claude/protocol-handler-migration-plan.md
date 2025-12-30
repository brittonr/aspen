# Protocol Handler Migration Plan

This plan tracks the extraction of client RPC handlers from monolithic `process_client_request` into modular, focused handlers.

## Overview

The goal is to replace the 2000+ line `process_client_request` function with a clean modular architecture where each handler is responsible for a specific domain.

## Architecture

```
ClientRpcRequest
       │
       ▼
┌──────────────────────────────────────────────────────────────┐
│                    HandlerRegistry                            │
│  (routes requests to appropriate handler based on variant)    │
└──────────────────────────────────────────────────────────────┘
       │
       ▼
┌──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┐
│ Core │  KV  │Coord │Lease │Clust │ SQL  │Watch │ Blob │ ...  │
└──────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┘
```

## Handler Status

### Fully Implemented (10/13) - 123 operations

| Handler | Operations | Status | Lines |
|---------|------------|--------|-------|
| CoreHandler | 9 | ✅ Complete | ~200 |
| KvHandler | 9 | ✅ Complete | ~400 |
| CoordinationHandler | 50 | ✅ Complete | ~1200 |
| LeaseHandler | 6 | ✅ Complete | ~265 |
| ClusterHandler | 12 | ✅ Complete | ~590 |
| WatchHandler | 3 | ✅ Complete | ~100 |
| ServiceRegistryHandler | 8 | ✅ Complete | ~510 |
| SqlHandler | 1 | ✅ Complete | ~185 |
| BlobHandler | 12 | ✅ Complete | ~900 |
| DocsHandler | 13 | ✅ Complete | ~720 |

### Placeholder Handlers (3/13)

| Handler | Operations | Priority | Complexity | Dependencies |
|---------|------------|----------|------------|--------------|
| DnsHandler | 6 | Low | Medium | DNS layer |
| ForgeHandler | 15+ | Low | High | Forge subsystem |
| PijulHandler | 10+ | Low | High | Pijul subsystem |

## Implementation Order

### Phase 1: Core Infrastructure (Complete)
- [x] Create handler trait and registry
- [x] Implement fallback to legacy `process_client_request`
- [x] CoreHandler (Ping, GetHealth, GetVersion, etc.)
- [x] KvHandler (ReadKey, WriteKey, DeleteKey, Scan, Batch ops)
- [x] CoordinationHandler (Locks, Counters, Sequences, RateLimiters, Barriers, Semaphores, RWLocks, Queues)

### Phase 2: Cluster Operations (Next)
- [x] LeaseHandler ✅
  - LeaseGrant, LeaseRevoke, LeaseKeepalive
  - LeaseTimeToLive, LeaseList, WriteKeyWithLease
- [x] ClusterHandler ✅
  - InitCluster, AddLearner, ChangeMembership, PromoteLearner
  - TriggerSnapshot, GetClusterState, GetClusterTicket, AddPeer
  - GetClusterTicketCombined, GetClientTicket, GetDocsTicket, GetTopology
- [x] WatchHandler ✅
  - WatchCreate, WatchCancel, WatchStatus
  - Note: Returns informative errors directing clients to LOG_SUBSCRIBER_ALPN streaming protocol

### Phase 3: Service Layer (Complete)
- [x] ServiceRegistryHandler ✅
  - ServiceRegister, ServiceDeregister
  - ServiceDiscover, ServiceList, ServiceGetInstance
  - ServiceHeartbeat, ServiceUpdateHealth, ServiceUpdateMetadata
- [x] SqlHandler ✅
  - ExecuteSql (with sql feature flag support)

### Phase 4: Content Layer
- [x] BlobHandler ✅
  - AddBlob, GetBlob, HasBlob, GetBlobTicket
  - ListBlobs, ProtectBlob, UnprotectBlob, DeleteBlob
  - DownloadBlob, DownloadBlobByHash, DownloadBlobByProvider, GetBlobStatus
- [x] DocsHandler ✅
  - DocsSet, DocsGet, DocsDelete, DocsList, DocsStatus
  - AddPeerCluster, RemovePeerCluster, ListPeerClusters
  - GetPeerClusterStatus, UpdatePeerClusterFilter
  - UpdatePeerClusterPriority, SetPeerClusterEnabled, GetKeyOrigin

### Phase 5: Domain Handlers
- [ ] DnsHandler
  - DnsRecordSet, DnsRecordGet, DnsRecordDelete
  - DnsRecordList, DnsZoneCreate, DnsZoneDelete
- [ ] ForgeHandler (complex, many operations)
- [ ] PijulHandler (complex, many operations)

## Files

- `src/protocol_handlers/mod.rs` - Handler trait and registry
- `src/protocol_handlers/handlers/` - Individual handler implementations
  - `core.rs` - CoreHandler ✅
  - `kv.rs` - KvHandler ✅
  - `coordination.rs` - CoordinationHandler ✅
  - `lease.rs` - LeaseHandler ✅
  - `cluster.rs` - ClusterHandler ✅
  - `watch.rs` - WatchHandler ✅
  - `sql.rs` - SqlHandler ✅
  - `blob.rs` - BlobHandler ✅
  - `docs.rs` - DocsHandler ✅
  - `dns.rs` - DnsHandler (placeholder)
  - `forge.rs` - ForgeHandler (placeholder)
  - `pijul.rs` - PijulHandler (placeholder)
  - `service_registry.rs` - ServiceRegistryHandler ✅

## Metrics

- **Total operations**: ~130+
- **Implemented**: 123 (95%)
- **Remaining**: 7+ (5%)

## Success Criteria

- [ ] All handlers implemented with no placeholders
- [ ] Legacy `process_client_request` removed or minimized
- [ ] All existing tests pass
- [ ] Clean separation of concerns
- [ ] Each handler < 500 lines

## Notes

- Handlers delegate to existing primitives (no reimplementation)
- Timeout values in RPC use 0 = no timeout convention
- Response types already defined in `client_rpc.rs`
- Use `validate_client_key()` for key validation
