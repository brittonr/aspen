## Context

We have `federation push` (sends local objects to remote) and `federation pull` (fetches remote objects locally). Both use the same wire protocol: `connect_to_cluster` → handshake, `get_remote_resource_state` → ref heads, `sync_remote_objects` → objects, `push_to_cluster` → push objects+refs. A bidirectional sync combines both over a single connection.

The existing `federation sync` CLI command (`FederationSyncPeer`) does discovery: handshake + list resources + store ref heads in KV. It does not transfer objects or update mirrors. The new `--repo` flag on `sync` will trigger bidirectional object transfer for a specific repo.

## Goals / Non-Goals

**Goals:**

- Single command to synchronize a repo between two clusters
- Reuse existing push/pull protocol — no new wire messages
- Pure function for ref comparison (verified module, testable)
- Pull-biased conflict resolution by default (remote wins on divergent refs)
- Combined stats: pulled N objects, pushed M objects, N refs updated each direction

**Non-Goals:**

- Three-way merge or conflict resolution on file contents (mirrors are whole-ref)
- Automatic periodic sync (that's `FederationSubscribe` territory)
- Sync all repos at once (one repo per invocation; script for bulk)
- New wire protocol messages (reuse `GetResourceState`, `SyncObjects`, `PushObjects`)

## Decisions

**1. Extend existing `sync` CLI command rather than adding a new subcommand**

`federation sync --peer <id>` already exists for discovery. Adding `--repo <hex>` triggers bidi sync for that specific repo. Without `--repo`, behavior is unchanged (discovery only). This keeps the CLI surface clean.

**2. Ref comparison as a pure function in `aspen-federation/src/verified/`**

```rust
pub struct RefDiff {
    pub to_pull: Vec<String>,   // refs where remote is ahead or local is missing
    pub to_push: Vec<String>,   // refs where local is ahead or remote is missing
    pub in_sync: Vec<String>,   // refs with matching hashes
    pub conflicts: Vec<String>, // refs diverged (different hashes, no ancestry info)
}

pub fn compute_ref_diff(
    local_heads: &HashMap<String, [u8; 32]>,
    remote_heads: &HashMap<String, [u8; 32]>,
) -> RefDiff
```

Without git ancestry info (we only have ref hashes, not commit graphs), diverged refs go to `conflicts`. The default resolution: remote wins (pull-biased). With `--push-wins`, local wins.

**3. Single RPC: `FederationBidiSync`**

```rust
FederationBidiSync {
    peer_node_id: String,
    peer_addr: Option<String>,
    repo_id: String,
    push_wins: bool,  // false = pull-biased (default)
}
```

Response combines pull and push stats:

```rust
FederationBidiSyncResponse {
    is_success: bool,
    pulled: u32,
    pushed: u32,
    pull_refs_updated: u32,
    push_refs_updated: u32,
    conflicts: Vec<String>,  // ref names that diverged
    errors: Vec<String>,
    error: Option<String>,
}
```

**4. Orchestration: connect once, pull first, push second**

1. Connect + handshake (one connection)
2. `get_remote_resource_state` → remote heads
3. Get local state via `ForgeResourceResolver` → local heads
4. `compute_ref_diff` → to_pull, to_push, conflicts
5. Resolve conflicts (remote wins or local wins based on flag)
6. Pull: `sync_remote_objects` for refs in to_pull set, import into mirror
7. Push: export local objects for refs in to_push set, `push_to_cluster`
8. Return combined stats

Pull happens before push so that if the connection drops mid-sync, we at least have the remote's updates.

## Risks / Trade-offs

**[Risk] Divergent refs have no ancestry info** → We can't tell if remote is ahead, behind, or forked without walking the commit graph. The `conflicts` list makes this visible. Default resolution (remote wins) matches `git fetch` behavior. Future improvement: use git SHA1 ancestry from `commit_sha1` field in `RefEntry` to detect fast-forwards.

**[Risk] Push requires trust on the remote side** → The remote's `handle_push_objects` checks trust. If Alice syncs with Bob but Bob doesn't trust Alice, the push half fails. The response reports this in `errors` and `pushed: 0` without failing the entire sync.

**[Risk] Large repos could exceed `MAX_OBJECTS_PER_SYNC` (1000) in a single round** → Both pull and push already paginate. The bidi sync just runs both pagination loops. Bounded by Tiger Style limits.
