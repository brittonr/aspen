## Context

Federation push (landed in `e019392`) sends git objects and ref updates from a local repo to a remote cluster. The receiver imports objects into a mirror repo. The reverse — pulling from a remote — partially exists: `handle_federation_fetch_refs` connects to a peer, calls `SyncObjects` for refs and git objects, and imports them into a local mirror. But the CLI only exposes this through `pull --repo <mirror-id>`, which requires a mirror to already exist (created by `sync --fetch`).

The wire protocol already supports everything needed. This is a plumbing change: extend the CLI and RPC layer to support cold-pull (no existing mirror), enrich mirror metadata for subsequent pulls, and add a proper integration test.

## Goals / Non-Goals

**Goals:**

- Pull a repo from a remote cluster by `--peer` + `--repo` (hex ID), creating a mirror if needed
- Subsequent `pull --repo <mirror-id>` reconnects using stored origin info (no `--peer` required)
- Incremental: only transfer objects not already in the local mirror
- Integration test: two-cluster create → pull → verify → push-new-commit → pull-again → verify-delta

**Non-Goals:**

- Automatic periodic pull / subscription (use `FederationSubscribe` for that)
- Pull by human-readable repo name (requires name resolution, separate change)
- Conflict resolution on divergent refs (mirror is read-only; just overwrite refs)
- New wire protocol messages (existing `GetResourceState` + `SyncObjects` + `PushObjects` suffice)

## Decisions

**1. Extend `FederationPull` RPC with optional `peer_node_id` and `peer_addr`**

The existing `FederationPull { mirror_repo_id }` stays as-is for the stored-mirror case. Add optional fields:

```rust
FederationPull {
    mirror_repo_id: Option<String>,  // was: String (breaking)
    peer_node_id: Option<String>,    // NEW
    peer_addr: Option<String>,       // NEW
    repo_id: Option<String>,         // NEW: remote repo hex ID
}
```

Two modes:

- `mirror_repo_id` set → existing behavior, look up stored metadata
- `peer_node_id` + `repo_id` set → cold pull, create mirror if needed

Alternative considered: separate `FederationPullRemote` RPC. Rejected — same underlying operation, just different ways to specify the target. One RPC with optional fields is simpler.

**2. Enrich `MirrorMetadata` with peer address hint**

Currently stores `origin_cluster_key` and `fed_id`. Add `origin_node_id` (iroh node ID for reconnection) and `origin_addr_hint` (optional socket address). The `origin_cluster_key` is the federation identity key; `origin_node_id` is the iroh transport key. Both are needed: identity for trust verification, node ID for connection.

**3. Reuse `handle_federation_fetch_refs` for the heavy lifting**

The cold-pull path resolves `--peer` + `--repo` into a `fed_id` (by querying `GetResourceState` on the remote), creates the mirror, stores metadata, then calls the same `handle_federation_fetch_refs` that the existing pull uses. No duplication.

**4. CLI: extend `Pull` subcommand args**

```
aspen-cli federation pull --peer <node-id> --repo <hex-id> [--addr <hint>]
aspen-cli federation pull --repo <mirror-id>
```

Both forms share the `Pull` subcommand. Validation: must provide either `--peer` + `--repo` (cold pull) or `--repo` alone (mirror pull). Ambiguity resolution: if `--peer` is set, `--repo` is the remote repo ID; otherwise it's the local mirror ID.

## Risks / Trade-offs

**[Risk] `FederationPull` field change is breaking** → The RPC is internal (iroh QUIC, not HTTP), and we don't maintain backwards compat. All fields use `Option<String>` with `#[serde(default)]` for wire safety.

**[Risk] Cold pull needs to construct FederatedId without knowing origin's cluster identity key** → The peer's node ID (iroh transport key) differs from their cluster identity key. During cold pull, we connect and handshake first (which returns the remote's `ClusterIdentity`), then construct the `FederatedId` from `identity.public_key()` + `repo_id`. This adds one extra round trip but keeps the protocol honest.

**[Risk] Mirror metadata scan is O(mirrors)** → Current code scans `_fed:mirror:*` to find a matching mirror. Acceptable at current scale (<1000 mirrors per Tiger Style bounds). Add a reverse index later if needed.
