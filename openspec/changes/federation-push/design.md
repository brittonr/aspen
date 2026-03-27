## Context

Federation sync is pull-only: the receiving cluster initiates a connection, handshakes, discovers resources, and fetches objects. The wire protocol (`FederationRequest`/`FederationResponse`), handler dispatch, resource resolver, and importer infrastructure all exist. The git bridge importer already supports wave-based parallel import of objects by SHA1 + type + raw bytes, and ref updates.

The push direction reuses all of this — the only new pieces are a `PushObjects` request variant, a handler that drives the importer, and a client function + CLI command that packages up the exporter output and sends it.

## Goals / Non-Goals

**Goals:**

- Origin cluster can push refs + git objects to a remote cluster over an existing federation connection
- Remote cluster validates trust/policy before accepting pushed objects
- Pushed objects are imported via the existing `GitObjectImporter` (wave-based, SHA1-keyed)
- CLI: `aspen-cli federation push --peer <key> --repo <id>`
- Integration test: two-cluster push with object verification

**Non-Goals:**

- Continuous/streaming push (webhook-style) — this is one-shot, like `git push`
- Conflict resolution for diverged refs — push fails if remote has newer refs (fast-forward only consideration deferred)
- Bidirectional merge — that's a higher-level workflow on top of push + pull
- Push to non-mirror repos — receiver creates or updates a mirror repo only

## Decisions

### 1. New `PushObjects` request variant on the existing wire protocol

Add `FederationRequest::PushObjects` carrying: `fed_id`, `objects: Vec<SyncObject>`, `ref_updates: Vec<RefEntry>`. The response is `FederationResponse::PushResult { accepted, imported, skipped, errors }`.

*Alternative: Separate ALPN for push.* Rejected — the federation ALPN already handles bidirectional streams, and push is just another request type in the same protocol.

### 2. Server handler calls `ForgeResolver::import_pushed_objects`

The handler in `handler.rs` matches `PushObjects`, checks trust (same as existing sync auth), then calls a new `import_pushed_objects` method on `FederationResourceResolver`. The Forge resolver implementation converts `SyncObject` items to `(Sha1Hash, GitObjectType, Vec<u8>)` tuples and delegates to `GitObjectImporter::import_objects` + `import_ref`.

*Alternative: Have the handler do the conversion.* Rejected — the resolver pattern keeps all app-specific logic behind the trait.

### 3. Client-side: `push_to_cluster` in sync/client.rs

New function: `push_to_cluster(conn, fed_id, objects, ref_updates) -> PushResult`. It builds a `PushObjects` request from exporter output and sends it over an existing connection (reusing `connect_to_cluster` + handshake).

### 4. Mirror repo creation on first push

When the receiver doesn't have a mirror repo for the pushed `fed_id`, it creates one. This mirrors the behavior of `federation fetch` which also creates mirror repos on first contact. The mirror repo ID is derived from the `fed_id` (deterministic).

### 5. RPC path: `FederationPush` client request → node handler → sync client

Same pattern as `FederationFetchRefs`: the CLI sends a `ClientRpcRequest::FederationPush` to the local node, which connects to the remote peer and calls `push_to_cluster`.

## Risks / Trade-offs

- **[Large push]** → Cap at existing `MAX_OBJECTS_PER_SYNC` (10,000). Client must chunk larger pushes.
- **[Trust asymmetry]** → Push requires the *receiver* to trust the *sender*, checked via existing `TrustManager`. A cluster can pull from anyone public but only accepts pushes from trusted peers.
- **[Ref conflict]** → First version does unconditional ref update (last-write-wins). Fast-forward enforcement is a follow-up.
