## Context

Federation sync already works end-to-end: Alice's cluster can serve refs and git objects via `ForgeResourceResolver::sync_objects`, and Bob's cluster can pull them via the `federation fetch` and `federation pull` CLI commands. Mirror repos store fetched refs/objects locally. The git-remote-aspen helper knows how to talk to a single cluster via `aspen://<ticket>/<repo_id>`.

The missing piece: git-remote-aspen can't address a repo through a *different* cluster's federation layer. The user has to manually run `federation fetch`, `federation pull`, then clone the local mirror — three steps that should be one `git clone`.

## Goals / Non-Goals

**Goals:**

- `git clone aspen://<ticket>/<fed-id>` works against Bob's cluster for a repo hosted on Alice's cluster
- `git fetch` on an already-cloned federated repo pulls incremental updates transparently
- The local cluster creates a mirror repo on first clone and reuses it on subsequent fetches
- Works with existing federation infrastructure (sync protocol, ForgeResourceResolver, mirror repos)

**Non-Goals:**

- Push through federation (federated `git push` — too complex for this change, requires write authorization across clusters)
- Automatic background mirror refresh (already exists via `MirrorConfig` / mirror worker)
- Multi-hop federation (A → B → C proxying)
- Discovery of federated repos by name (user must know the `fed-id`)

## Decisions

### D1: URL encoding for federated repos

**Decision**: Encode the federated identity in the repo_id portion of the URL using a `fed:` prefix.

```
aspen://<local-cluster-ticket>/fed:<origin-pubkey-base32>:<repo-id-hex>
```

Example: `aspen://<ticket>/fed:ab3k7x...:aabb11...`

**Why**: Reuses the existing URL parser with minimal changes. The `fed:` prefix clearly distinguishes federated from local repos. The origin public key identifies which cluster to pull from. The repo-id-hex identifies the repo on that cluster.

**Alternative considered**: Separate `aspen-fed://` scheme — rejected because it requires changes to git's remote helper discovery and complicates the binary name.

**Alternative considered**: Encode fed-id as a special ticket type — rejected because the user still needs their *local* cluster ticket for authentication/routing.

### D2: RPC flow — local cluster proxies the request

**Decision**: git-remote-aspen sends standard `GitBridgeListRefs` / `GitBridgeFetch` to the local cluster with a federation wrapper. The local node detects the federated repo ID, does a federation sync to the origin cluster, imports into a mirror, then responds to the git client from the mirror.

```
git client
  → git-remote-aspen (parses fed: URL)
    → local cluster (FederationGitListRefs RPC)
      → origin cluster (federation sync_objects)
      ← refs + objects
    ← mirror refs to git client
  → git-remote-aspen (fetch objects)
    → local cluster (FederationGitFetch RPC)
      ← objects from mirror (already imported)
    ← loose objects written to .git/
```

**Why**: The git-remote-aspen binary stays thin (no federation logic). The cluster node has all the federation infrastructure already wired. The mirror persists objects so subsequent fetches are fast.

**Alternative considered**: git-remote-aspen connects directly to both clusters — rejected because federation requires trust verification, ALPN routing, and sync protocol that the helper binary doesn't have.

### D3: Two new RPC variants

**Decision**: Add `FederationGitListRefs` and `FederationGitFetch` to `ClientRpcRequest`.

- `FederationGitListRefs { origin_key, repo_id, origin_addr_hint }` → returns refs (same shape as `GitBridgeListRefsResponse`)
- `FederationGitFetch { origin_key, repo_id, want, have, origin_addr_hint }` → returns objects (same shape as `GitBridgeFetchResponse`)

The handler:

1. Checks if a mirror already exists for this federated repo
2. If not, creates one and does a full federation sync
3. If yes, does an incremental federation pull if the mirror is stale (>30s since last sync)
4. Serves the response from the local mirror using the existing git bridge exporter

**Why**: Separate RPC variants keep the existing `GitBridgeListRefs`/`GitBridgeFetch` clean. The handler encapsulates all federation complexity.

### D4: Mirror lifecycle

**Decision**: On first `FederationGitListRefs`, create a mirror repo with `MirrorConfig` pointing to the origin. On `FederationGitFetch`, serve from the mirror. The mirror is a normal Forge repo that happens to have a `MirrorConfig`.

The mirror repo ID is derived deterministically: `blake3(origin_pubkey || upstream_repo_id)`. This ensures the same federated repo always maps to the same local mirror.

**Why**: Deterministic mirror IDs prevent duplicate mirrors. Reusing the existing `MirrorConfig` infrastructure means the mirror worker can keep it updated in the background after the initial clone.

### D5: Staleness check before serving

**Decision**: On each `FederationGitListRefs`, if the mirror's `last_sync_ms` is older than 30 seconds, do a federation pull before responding. This is a simple freshness guarantee without requiring the mirror worker to be running.

**Why**: The user expects `git fetch` to return current refs. 30s is short enough to feel fresh but long enough to avoid hammering the origin on rapid git operations (list → fetch happen in quick succession).

## Risks / Trade-offs

- **[First clone latency]** → The initial clone must do a full federation sync before responding. For large repos this could take 10-30s. Mitigation: the git-remote-aspen helper already has a 600s RPC timeout; stderr progress messages will show what's happening.
- **[Origin unreachable]** → If Alice's cluster is down, the clone fails. After initial clone, fetches succeed from the stale mirror with a warning. Mitigation: return the stale mirror data with a stderr warning rather than failing.
- **[Mirror disk usage]** → Every federated clone creates a full local copy. Mitigation: same tradeoff as `git clone` itself — this is expected behavior. Mirror cleanup is future work.
- **[RPC size limits]** → Large repos may exceed single-RPC limits. Mitigation: the federation `sync_objects` already supports pagination via `have_hashes`; the handler can do multiple rounds internally.
