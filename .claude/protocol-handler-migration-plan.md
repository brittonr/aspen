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

### Fully Implemented (4/13) - 74 operations

| Handler | Operations | Status | Lines |
|---------|------------|--------|-------|
| CoreHandler | 9 | ✅ Complete | ~200 |
| KvHandler | 9 | ✅ Complete | ~400 |
| CoordinationHandler | 50 | ✅ Complete | ~1200 |
| LeaseHandler | 6 | ✅ Complete | ~265 |

### Placeholder Handlers (9/13)

| Handler | Operations | Priority | Complexity | Dependencies |
|---------|------------|----------|------------|--------------|
| ClusterHandler | 8 | High | Low | ClusterController trait |
| WatchHandler | 3 | High | Medium | Watch infrastructure |
| ServiceRegistryHandler | 8 | Medium | Low | Existing ServiceRegistry |
| SqlHandler | 2 | Medium | Low | DataFusion executor |
| BlobHandler | 6 | Medium | Medium | iroh-blobs |
| DocsHandler | 8 | Medium | Medium | iroh-docs |
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
- [ ] ClusterHandler
  - ClusterInit, AddLearner, ChangeMembership
  - GetMetrics, TriggerSnapshot, GetLeader, ClusterStatus
- [ ] WatchHandler
  - WatchCreate, WatchCancel, WatchStatus

### Phase 3: Service Layer
- [ ] ServiceRegistryHandler
  - ServiceRegister, ServiceDeregister
  - ServiceDiscover, ServiceHeartbeat
  - ServiceHealthUpdate, ServiceMetadataUpdate
  - ServiceList
- [ ] SqlHandler
  - SqlQuery, SqlExplain

### Phase 4: Content Layer
- [ ] BlobHandler
  - BlobPut, BlobGet, BlobDelete
  - BlobList, BlobStat, BlobShare
- [ ] DocsHandler
  - DocCreate, DocOpen, DocClose
  - DocGet, DocSet, DocDelete
  - DocSubscribe, DocSync

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
  - `lease.rs` - LeaseHandler (placeholder)
  - `cluster.rs` - ClusterHandler (placeholder)
  - `watch.rs` - WatchHandler (placeholder)
  - `sql.rs` - SqlHandler (placeholder)
  - `blob.rs` - BlobHandler (placeholder)
  - `docs.rs` - DocsHandler (placeholder)
  - `dns.rs` - DnsHandler (placeholder)
  - `forge.rs` - ForgeHandler (placeholder)
  - `pijul.rs` - PijulHandler (placeholder)
  - `service_registry.rs` - ServiceRegistryHandler (placeholder)

## Metrics

- **Total operations**: ~130+
- **Implemented**: 74 (57%)
- **Remaining**: 56+ (43%)

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
