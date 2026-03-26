## Context

Federation sync currently stops at ref metadata. The `federation fetch` command connects to a remote cluster, discovers forge repos, fetches ref entries (name → commit hash pairs), and stores them in local KV under `_fed:mirror:` keys. But the git objects those hashes point to (commits, trees, blobs) never transfer. A local user can see that a remote repo has `heads/main → abc123` but can't read or clone it.

The forge already has a complete git bridge with import/export capabilities (`GitBridgeImporter`, `GitBridgeExporter`). Objects are stored as iroh-blobs (BLAKE3 content-addressed), with SHA1↔BLAKE3 hash mappings in KV. The exporter can walk a commit DAG and produce git objects; the importer can ingest git objects and store them as blobs with mappings.

The federation protocol already has a `SyncObjects` request/response that sends `SyncObject { object_type, hash, data }` over QUIC streams. The `ForgeResourceResolver` only returns `"ref"` type objects today.

## Goals / Non-Goals

**Goals:**

- Transfer git objects (commits, trees, blobs) for federated refs via iroh-blobs
- Create local mirror repos that are readable via standard forge operations and git clone
- Incremental sync — only transfer objects the local cluster doesn't have
- Content verification — BLAKE3 hash check on all transferred objects

**Non-Goals:**

- Writable mirrors (mirrors are read-only; writes go to origin cluster)
- Automatic background sync (this is on-demand via CLI; periodic sync is a later feature)
- Large file / LFS support (standard git objects only for now)
- Pack file format over the wire (transfer individual git objects, not git pack streams — simpler, content-addressable, and iroh-blobs handles chunking/resumption)

## Decisions

### 1. Transfer individual git objects, not packfiles

**Decision**: Send git objects individually as `SyncObject` entries with `object_type: "commit"`, `"tree"`, `"blob"`. Each object's data is the raw git object bytes.

**Why not packfiles**: Packfiles are opaque binary streams that need repacking on the receiver. Individual objects are content-addressed (BLAKE3), can be verified independently, deduplicated by hash, and map directly to iroh-blobs entries. The existing `GitBridgeExporter::export_commit_dag` already walks the DAG and produces individual objects.

**Trade-off**: More round trips for large repos. Mitigated by batching objects per `SyncObjects` request (up to `MAX_OBJECTS_PER_SYNC = 1000`) and paging via `has_more`.

### 2. Use the existing SyncObjects protocol, not a new message type

**Decision**: Extend `ForgeResourceResolver::sync_objects` to serve `"commit"`, `"tree"`, `"blob"` types in addition to `"refs"`. The client requests `want_types: ["refs", "commit", "tree", "blob"]` and the server walks from ref heads to return reachable objects.

**Rationale**: The protocol already handles batching, have_hashes dedup, and content verification. No wire format changes needed.

### 3. Server-side DAG walk with have_hashes exclusion

**Decision**: The server receives `have_hashes` from the client (BLAKE3 hashes of objects the client already has), walks the commit DAG from ref heads, and stops traversal at known objects. Returns only missing objects.

**Rationale**: Matches git's own fetch negotiation pattern. The existing `export_commit_dag` method already accepts `known_to_remote` for this purpose (though it uses SHA1 — we'll add a BLAKE3 variant).

### 4. Mirror repos as regular forge repos with federation metadata

**Decision**: Create a local forge repo via the standard `ForgeNode::create_repo` path, import the fetched objects via `GitBridgeImporter::import_objects`, then tag the repo in KV as `_fed:mirror:{fed_id}` so it's identifiable as a mirror.

**Rationale**: Reuses all existing forge infrastructure (git bridge, blob store, ref management). Mirror repos are first-class repos that work with `git clone aspen://...` and all forge read operations.

### 5. Two-phase fetch: refs first, then objects

**Decision**: `federation fetch` does:

1. `SyncObjects(want_types: ["refs"])` → get current ref state
2. Diff against local mirror refs to find new/changed refs
3. `SyncObjects(want_types: ["commit", "tree", "blob"], have_hashes: [local objects])` → get missing objects
4. Import objects into local mirror repo
5. Update local mirror refs

**Rationale**: Separating ref discovery from object transfer lets the client compute exactly what it needs. Ref entries are small (postcard-serialized name+hash), so fetching all of them is cheap.

## Risks / Trade-offs

- **Large repos**: A first sync of a large repo transfers all reachable objects. Mitigated by pagination (`has_more`) and the 1000-object batch limit. Could add depth-limiting later.
- **Server memory for DAG walk**: Walking a deep commit graph loads object metadata. Mitigated by the existing `MAX_DAG_DEPTH` constant in the git bridge and the `MAX_OBJECTS_PER_SYNC` cap.
- **SHA1↔BLAKE3 mapping gap**: The server's `export_commit_dag` uses SHA1 for `known_to_remote`. The federation protocol uses BLAKE3 hashes in `have_hashes`. The server needs to translate — look up BLAKE3→SHA1 mappings. If a mapping is missing, the object gets re-sent (safe, just wasteful).
- **Mirror staleness**: Mirrors don't auto-update. Users must run `federation fetch` again. Acceptable for v1; background sync subscriptions can come later.
