## Context

Federated git clone syncs a repo from an origin cluster (alice) to a mirror on the local cluster (bob) via the federation protocol, then serves the mirror's objects to git-remote-aspen. The pipeline has three layers:

1. **Transport**: QUIC streams between clusters via iroh, carrying length-prefixed postcard messages.
2. **Object transfer**: Multi-round `SyncObjects` RPCs. Each round sends a batch of git objects (blobs, trees, commits). The origin's `ForgeResourceResolver` walks the commit DAG via `export_dag_blake3`, skipping objects the mirror already has.
3. **Import**: `federation_import_objects` on the mirror converts SyncObjects to Forge internal format (`SignedObject<GitObject>`). Trees have their SHA-1 entry references resolved to BLAKE3 envelope hashes via the mapping store. The convergent loop retries failed objects until no progress is made.

The bug: after all 33,897 objects are imported (confirmed `missed=0` in Phase 4), the git bridge fetch's BFS from HEAD reaches only 7,564 objects. The Forge DAG is structurally truncated. The walk completes cleanly — no errors, no "unresolvable BLAKE3" warnings — because the remaining objects exist in KV but aren't reachable from any tree entry's BLAKE3 reference.

## Goals / Non-Goals

**Goals:**

- Federated clone of the Aspen workspace (33K objects) succeeds end-to-end.
- The federation sync uses a single persistent QUIC stream per conversation (idiomatic iroh).
- The mirror's Forge DAG is structurally complete after import — every BLAKE3 reference resolves.

**Non-Goals:**

- Optimizing transfer size (pack files, delta compression). Separate concern.
- Bidirectional federation push. Only pull (clone from origin) is in scope.
- Changing the federation wire protocol version. The `FederationRequest::SyncObjects` message shape stays the same.

## Decisions

### Decision 1: Single-stream sync conversation

**Choice**: Replace open-stream-per-RPC with a persistent bidirectional stream held for the entire multi-round sync.

**Current state**: Each `sync_remote_objects` call does `connection.open_bi()`, writes one request, calls `send.finish()`, reads one response. The server's `handle_federation_connection` loop accepts each stream, spawns a one-shot handler. After 16 total streams the connection was killed (now semaphore-limited but still wasteful).

**New design**:

- Client: `SyncSession::new(connection)` opens one `open_bi()`. Methods like `sync_objects()` write a request and read a response on the same stream pair. The session is dropped (calling `send.finish()`) when the multi-round loop ends.
- Server: `handle_federation_stream` becomes a loop: read request → process → write response → read next request → ... until the client finishes the send side (detected as `read_message` returning EOF / connection reset).
- The handshake still uses its own stream (first stream on the connection). The sync conversation is the second stream.

**Why not keep stream-per-RPC with higher limits?** Stream-per-RPC is the HTTP/1.1 pattern. QUIC bidirectional streams are designed for long-lived conversations. Reusing a single stream avoids stream accounting entirely, reduces overhead (no stream setup per round), and matches how iroh's own protocols (iroh-blobs, iroh-docs) work.

**Alternatives considered**: Increasing `MAX_STREAMS_PER_CONNECTION` to 1000. Simple but treats the symptom. The semaphore fix (already applied) helps but still opens/closes streams per round unnecessarily.

### Decision 2: Post-import DAG re-resolution

**Choice**: After the final retry pass, walk every ref head's tree DAG on the mirror and re-resolve stale BLAKE3 references.

**Root cause**: When a tree T is imported in round N, its entry for sub-object S is resolved as `sha1(S) → blake3(S)` using the mapping store at round N. If S was imported in round N with envelope hash H1, the tree stores H1. Later, in the retry pass, S is skipped (already has a SHA-1 mapping), so H1 remains correct. BUT — if T *itself* was imported in round N while one of its entries' sub-objects hadn't been imported yet, T's import failed. T is retried in the final pass where S is available. T's BLAKE3 reference for S is set correctly at retry time.

The problem is trees that *succeeded* in early rounds but reference sub-objects whose BLAKE3 is from early-round import. If those sub-objects were later re-imported differently (or if the exporter uses the origin's BLAKE3 rather than the mirror's), the reference goes stale.

**Approach**: After the final retry pass in `federation_import_objects`:

1. Scan all ref heads in the mirror.
2. BFS from each ref head, reading each tree's entries.
3. For each entry, check `read_object_bytes(entry_blake3)` succeeds.
4. If not, look up the entry's SHA-1 in the mapping store to get the current BLAKE3 and update the tree entry.
5. Re-store the tree with corrected references. This changes the tree's envelope hash, so update the parent's reference too (propagate up).

**Alternative**: Re-import all trees after the final retry (discard and rebuild). Simpler but expensive — every tree's SignedObject would get a new BLAKE3, cascading changes up to the commit. The targeted re-resolution is cheaper.

**Alternative**: Import everything in a single mega-batch instead of multi-round. Would avoid the issue but requires the origin to serialize the entire DAG in one response (potentially hundreds of MB). Breaks the streaming design.

### Decision 3: Origin SHA-1 cross-pass resolution (already implemented)

**Choice**: Store `origin_sha1 → blake3` mappings after each convergent pass, not only in Phase 4.

This is already applied in the working tree. It reduced stuck objects from 181 to 0 in the convergent import. It's necessary but not sufficient — the DAG truncation persists because trees imported in early rounds have BLAKE3 references set before later rounds add more mappings.

## Risks / Trade-offs

- **[Post-import re-resolution is O(objects)]** → Bounded by `MAX_GIT_OBJECTS_PER_PUSH` (50K). For a 33K-object repo this is ~5-10 seconds of KV reads. Acceptable for a one-time sync operation.
- **[Re-resolution changes tree envelope hashes]** → Fixing a tree's entry BLAKE3 changes the tree's own SignedObject bytes, changing its envelope hash. Parent trees/commits referencing this tree need updating too. This cascading update must propagate from leaves to root. → Mitigate by processing in reverse topological order (blobs → trees → commits).
- **[Single-stream server loop could block on one slow request]** → Each request is processed sequentially on the stream. A slow DAG walk blocks subsequent requests. → This is fine because the client waits for each response before sending the next request (synchronous conversation). No parallelism is lost.
- **[Wire compatibility]** → The message format doesn't change. A new client talking to an old server that expects one-shot streams would fail when the server closes the stream after one response. → Version negotiation: include `"streaming-sync"` in handshake capabilities. Fall back to stream-per-RPC if the peer doesn't advertise it.

## Open Questions

1. Should the re-resolution pass run on every federation import, or only when the convergent loop had failures (indicating cross-batch dependency issues)?
2. Should the single-stream pattern extend to all federation RPCs (push, ref queries), or only the multi-round sync?
