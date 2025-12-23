# CID-Router Evaluation for Aspen

**Date**: 2025-12-22
**Repository**: https://github.com/eqtylab/cid-router
**License**: Apache-2.0

## Executive Summary

**Recommendation: Not currently useful for Aspen, but worth monitoring.**

CID-router solves a different problem than Aspen faces. It's a content-routing layer that maps CIDs (Content Identifiers) to retrieval routes across multiple storage backends (Azure Blob, Iroh blobs, etc.). Aspen already has a mature, tightly-integrated iroh-blobs implementation that doesn't need multi-backend routing.

### Key Findings

| Aspect | CID-Router | Aspen Current | Gap |
|--------|------------|---------------|-----|
| **Hash Format** | CID (multihash + multicodec) | BLAKE3 Hash | Aspen uses raw hashes, not CIDs |
| **Storage Backend** | Multi-provider (Azure, Iroh, extensible) | Single iroh-blobs FsStore | Aspen is single-backend |
| **Network Layer** | HTTP API (Axum) | Iroh QUIC + ALPN | Architectural mismatch |
| **Routing Logic** | SQLite-backed route registry | Implicit via KV/docs sync | Different discovery models |
| **Consensus** | None (centralized) | Raft via openraft | Aspen is distributed-first |
| **Provider Discovery** | Configuration-driven indexing | Gossip + docs sync | Different discovery paradigms |

## Detailed Analysis

### What CID-Router Does

CID-router provides:

1. **Multi-Provider Content Routing**: Maps CIDs to routes from multiple storage backends
2. **Pluggable CRP (CID Route Provider) Trait**: Extensible architecture for adding new backends
3. **Two-Phase Indexing**: RouteStub -> Route workflow for async content verification
4. **HTTP API**: REST endpoints for route queries and blob retrieval
5. **Iroh Integration**: Uses iroh-blobs 0.97 for BLAKE3 content storage

### Core Architecture

```
CID-Router Architecture:
┌─────────────────────────────────────────────────────┐
│          HTTP API (Axum + Swagger)                  │
└────────────────────┬────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────┐
│         Core Context + SQLite Route DB             │
└────────────────────┬────────────────────────────────┘
                     ↓
         ┌───────────┼───────────┐
         ↓           ↓           ↓
    ┌────────┐  ┌────────┐  ┌────────┐
    │ Iroh   │  │ Azure  │  │ Future │
    │ CRP    │  │ CRP    │  │ CRPs   │
    └────────┘  └────────┘  └────────┘
```

### Aspen's Current Architecture

```
Aspen Architecture:
┌─────────────────────────────────────────────────────┐
│     Iroh QUIC (CLIENT_ALPN, RAFT_ALPN, BLOBS)      │
└────────────────────┬────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────┐
│   BlobAwareKeyValueStore (transparent offloading)  │
└────────────────────┬────────────────────────────────┘
                     ↓
         ┌───────────┼───────────┐
         ↓                       ↓
┌─────────────────┐     ┌─────────────────┐
│  IrohBlobStore  │     │    RaftNode     │
│  (iroh-blobs)   │     │  (consensus)    │
└─────────────────┘     └─────────────────┘
```

### Why CID-Router Doesn't Fit

#### 1. **Different Hash Semantics**

- CID-router uses full CIDs (multicodec + multihash)
- Aspen uses raw BLAKE3 hashes via iroh-blobs
- Converting would add unnecessary complexity for no benefit

#### 2. **HTTP vs Iroh Network Stack**

- CID-router exposes HTTP endpoints
- Aspen uses Iroh QUIC with ALPN-based protocol routing
- Adding HTTP would violate Aspen's "Iroh-First, No HTTP" principle

#### 3. **Centralized vs Distributed**

- CID-router uses SQLite for route storage (single instance)
- Aspen is Raft-replicated across cluster nodes
- CID-router would need significant rework for distributed deployment

#### 4. **Single Backend is Sufficient**

- Aspen only needs iroh-blobs
- No current requirement for Azure, S3, or other backends
- Multi-provider routing adds complexity without value

#### 5. **Provider Discovery Model Mismatch**

- CID-router: Configuration-driven reindexing (hourly scans)
- Aspen: Gossip-based peer discovery + docs sync for content
- Different paradigms that don't compose well

### Technical Comparison: Iroh Integration

**CID-Router's Iroh CRP:**

```rust
pub struct IrohCrp {
    store: iroh_blobs::store::fs::FsStore,  // No networking
}

impl Crp for IrohCrp {
    fn cid_filter(&self) -> CidFilter {
        CidFilter::MultihashCodeFilter(CodeFilter::Eq(0x1e))  // BLAKE3 only
    }
}
```

**Aspen's Iroh Integration:**

```rust
pub struct IrohBlobStore {
    store: FsStore,
    endpoint: Endpoint,  // Full networking support
}

impl BlobStore for IrohBlobStore {
    async fn download_from_peer(&self, hash: &Hash, provider: PublicKey);
    async fn ticket(&self, hash: &Hash) -> BlobTicket;
}
```

Aspen's integration is more complete:

- P2P networking via Endpoint
- Ticket generation for sharing
- Direct peer downloads
- Protocol handler for ALPN routing

### Potential Future Relevance

CID-router could become relevant if Aspen needs:

1. **Multi-Cluster Federation**: Content routing across separate Aspen clusters
2. **IPFS Interoperability**: Gateway compatibility with IPFS tooling
3. **External Storage Backends**: S3, Azure, or other cloud storage
4. **Content Marketplace**: Provider reputation and selection

None of these are current requirements.

### Alternative Approaches

If Aspen eventually needs enhanced content routing:

1. **Extend IrohBlobStore**: Add provider selection logic internally
2. **Gossip-Based Content Advertisement**: Use existing gossip for "I have this blob"
3. **Raft-Replicated Route Registry**: Store routes in KV store, not external SQLite
4. **CID Conversion Layer**: Thin wrapper to convert BLAKE3 hashes to CIDs for external APIs

### Code Quality Assessment

CID-router has some quality concerns:

| Issue | Severity | Notes |
|-------|----------|-------|
| Many `.unwrap()` calls | Medium | Should use `?` operator |
| Route signing stub | Medium | `sign_route()` returns empty vec |
| Single SQLite connection | Medium | No connection pooling |
| No Iroh reindexing | Low | TODO in iroh.rs |
| HTTP-centric design | N/A | Architectural choice |

### Dependency Versions

```toml
# CID-Router
iroh = "0.95"
iroh-base = "0.95"
iroh-blobs = "0.97"

# Aspen (estimated from context)
iroh-blobs = "0.32" or similar  # Older version
```

Version mismatch would require evaluation before any integration.

## Recommendations

### Short Term (Current)

- **No action required**: Aspen's blob storage is production-ready
- **Monitor repository**: Watch for features that might become relevant

### Medium Term (If Needed)

- **Implement gossip-based content advertisement** before considering external tools
- **Add multi-source download** natively in IrohBlobStore
- **Evaluate CID adoption** only if IPFS interop becomes a requirement

### Long Term (Federation)

- **Consider CID-router patterns** (not codebase) if building multi-cluster content routing
- **Prefer Raft-replicated route storage** over SQLite
- **Maintain Iroh-native networking** rather than adding HTTP

## Files Analyzed

### CID-Router

- `core/src/crp.rs`: CRP trait definition
- `core/src/routes.rs`: Route and RouteStub types
- `core/src/db.rs`: SQLite route storage
- `crps/iroh/src/iroh.rs`: Iroh CRP implementation
- `server/src/api/v1/data.rs`: HTTP API endpoints
- `Cargo.toml`: Dependencies and workspace structure

### Aspen

- `src/blob/store.rs`: IrohBlobStore implementation
- `src/blob/types.rs`: BlobRef and related types
- `src/blob/kv_integration.rs`: BlobAwareKeyValueStore
- `src/blob/constants.rs`: Tiger Style resource bounds
- `src/cluster/gossip_discovery.rs`: Peer discovery via gossip

## Conclusion

CID-router is a well-designed multi-backend content routing system, but it solves a different problem than Aspen faces. Aspen's single-backend, Iroh-native, Raft-replicated architecture is a better fit for its use case. The architectural differences (HTTP vs QUIC, centralized vs distributed, CID vs raw hash) make integration more costly than beneficial.

**Verdict**: Not useful for Aspen's current or near-future needs. The patterns (pluggable CRP trait, two-phase indexing) are interesting but would need to be reimplemented with Aspen's architectural constraints rather than adopted wholesale.
