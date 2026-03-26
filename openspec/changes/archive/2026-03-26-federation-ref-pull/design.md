## Context

Federation sync discovers remote repositories and persists ref heads to local KV, but the git objects (commits, trees, blobs) behind those refs are never transferred. The `sync_remote_objects` client function and `SyncObjects` protocol message already exist in the wire protocol. The `ForgeResourceResolver::sync_objects` returns an empty vec today — it acknowledges that "actual git objects are transferred via iroh-blobs" but doesn't implement it.

Two paths exist for object transfer:

1. **SyncObjects protocol** — federation wire protocol sends objects inline as `SyncObject { data, hash }`. Already defined, verified with BLAKE3 + delegate signatures.
2. **iroh-blobs** — content-addressed blob transfer. The forge already stores git objects in iroh-blobs (packfiles).

The `ForgeResourceResolver` comment says objects go via iroh-blobs, but the federation protocol already has `SyncObjects` with verification. For refs metadata (small objects like commit/tree pointers), the federation protocol is the right path. For large pack data, iroh-blobs is better. This design uses the federation `SyncObjects` for ref-associated metadata and stores results locally.

## Goals / Non-Goals

**Goals:**

- `federation fetch` CLI command that pulls git objects for previously synced refs
- `ForgeResourceResolver::sync_objects` returns actual ref data from KV
- Fetched objects are persisted locally so the repo can be read
- `federation sync --fetch` combines discovery + fetch in one command
- NixOS VM test proving cross-cluster fetch works end-to-end

**Non-Goals:**

- Incremental/streaming pack transfer (future work with iroh-blobs)
- Automatic background sync (separate change)
- Push-based federation (reverse direction)
- Conflict resolution for divergent refs

## Decisions

1. **Use federation SyncObjects for ref state transfer.** The wire protocol already has `SyncObjects` with BLAKE3 verification. For the initial implementation, the "objects" are ref entries (ref_name → hash mappings). This keeps the implementation contained within the federation protocol without adding iroh-blobs coordination. Actual git packfile transfer via iroh-blobs is future work.

2. **ForgeResourceResolver returns ref entries as SyncObjects.** Each ref becomes a `SyncObject { object_type: "ref", hash: blake3(ref_name + head_hash), data: postcard(RefEntry) }`. The `have_hashes` filter lets the requester skip refs it already has.

3. **Fetched refs are stored under `forge:refs:{repo_id}:{ref_name}` with a `_fed:mirror:` prefix.** Local refs keep their namespace; federated mirrors get `_fed:mirror:{origin_short}:{repo_id}:refs/{name}` to avoid collisions with locally-owned refs.

4. **New `FederationFetchRefs` RPC message.** A dedicated RPC rather than reusing `FederationSyncPeer`, because fetch has different semantics (writes to local store) and needs a repo-level scope.

5. **The `--fetch` flag on `sync` calls sync then fetch in sequence.** No new RPC — the CLI orchestrates both calls.

## Risks / Trade-offs

- **SyncObjects for refs only** — this doesn't transfer actual git object content (blobs, commits, trees). The local cluster gets ref pointers but can't `git clone` the mirrored repo until pack transfer is added. This is an incremental step.
- **No deduplication across federation sources** — if two remote clusters federate the same repo, we store refs from both. Acceptable at current scale.
- **KV storage for ref mirrors** — ref state is small (tens of bytes per ref). At 1000 federated repos × 100 refs each = ~3MB of KV. Well within bounds.
