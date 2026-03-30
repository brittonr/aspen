## Context

The git bridge push path already has a chunked protocol (`GitBridgePushStart/Chunk/Complete`) where the client sends objects in 4 MB batches. Each batch is a separate RPC round-trip. The server accumulates objects, imports them, and responds with results after `Complete`.

The fetch direction currently uses a single `GitBridgeFetch` RPC that returns ALL objects inline. This works for repos up to ~10K objects (~15 MB serialized). Beyond that, the serialized response exceeds what QUIC can reliably deliver in one write.

The fetch path has two callers:

1. **git-remote-aspen** (external client) — talks to the server via CLIENT_ALPN
2. **federation_git.rs** (internal) — calls `handle_git_bridge_fetch` directly within the server process

For (1), the chunked protocol uses separate RPC round-trips. For (2), the internal call can use the same handler but needs the response assembled locally.

## Goals / Non-Goals

**Goals:**

- Fetch 33K+ object repos without OOM or write failure.
- Incremental loose object writing — git-remote-aspen writes objects per chunk, never holding all 490 MB in memory.
- Backward compatible — old clients that send `GitBridgeFetch` still get the single-shot response for small repos.

**Non-Goals:**

- Git pack file generation (delta compression). Separate optimization.
- Streaming via a single QUIC stream (multi-message). The existing RPC model uses one stream per request/response. Chunked fetch uses multiple RPCs, same as chunked push.

## Decisions

### Decision 1: Mirror the push protocol

**Choice**: `GitBridgeFetchStart { repo_id, want, have }` → server responds with `FetchStartResponse { session_id, total_objects, total_chunks }`. Then client sends `GitBridgeFetchChunk { session_id, chunk_id }` → server responds with `FetchChunkResponse { objects, chunk_hash }`. Finally client sends `GitBridgeFetchComplete { session_id }`.

**Why**: Consistent with the push protocol. The server holds the session state (DAG walk result), batches it, and serves chunks on demand. The client drives the pace.

**Alternative**: Server-push streaming (server sends all chunks without client polling). Simpler but doesn't fit the existing request/response RPC model without adding a new streaming primitive.

### Decision 2: Server-side session holds the full DAG walk result

**Choice**: On `FetchStart`, the server runs the full DAG walk (like current `GitBridgeFetch`), stores the result in a temporary session keyed by `session_id`, and reports `total_objects` and `total_chunks`. Each `FetchChunk` request serves the next batch from the session.

**Why**: The DAG walk is the expensive part (~5-10s for 33K objects). Running it once and caching the result avoids re-walking per chunk. Sessions expire after a timeout (e.g., 5 minutes).

**Risk**: Memory — holding 33K `ExportedObject`s in memory (~490 MB raw content). Mitigate by only holding the (blake3, sha1, object_type) metadata in the session, re-reading object content from KV on each chunk request. This trades I/O for memory.

### Decision 3: Threshold for chunked vs single-shot

**Choice**: If the DAG walk produces ≤ 2,000 objects, return them inline via the existing `GitBridgeFetch` response. If > 2,000, use the chunked protocol.

**Why**: 2,000 objects at ~2 KB avg = ~4 MB, well within the 16 MB RPC limit. The threshold avoids unnecessary round-trips for small repos.

### Decision 4: Federation internal path

**Choice**: `handle_federation_git_fetch` calls the same chunked handler internally. Since it runs within the same process, it can accumulate chunks in-memory without RPC overhead.

**Alternative**: Use a different code path for internal vs external callers. Adds complexity without benefit — the chunked handler works for both.

## Risks / Trade-offs

- **[Session memory]** → Each session holds DAG walk metadata (~33K entries × ~100 bytes = ~3 MB). Acceptable. Raw content is re-read from KV per chunk.
- **[Session timeout race]** → If the client is slow, the session expires. → Include `session_id` expiry in the response so the client knows the deadline. Client can request a new session.
- **[Chunk ordering]** → Chunks must be written to `.git/objects/` in dependency order (blobs first). The server batches by type: blobs → trees → commits, same as push batching.

## Open Questions

1. Should the federation internal path use the chunked RPC variants or a direct in-process iterator?
