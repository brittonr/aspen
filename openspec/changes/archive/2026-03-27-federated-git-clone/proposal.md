## Why

`git clone aspen://<ticket>/<repo>` only works against the cluster that owns the repo. Federation built the primitives (handshake, ref pull, blob transfer, mirror repos) but never wired them into the git-remote-aspen helper. A user on Bob's cluster can't `git clone` a repo that lives on Alice's cluster — they'd have to get Alice's ticket directly. Federated git clone closes this gap: Bob's cluster acts as a transparent proxy, fetching refs and objects from Alice on demand, so standard git commands work against any federated repo regardless of which cluster hosts it.

## What Changes

- **New URL scheme**: `aspen://<local-ticket>/<fed-id>` where `fed-id` encodes origin cluster + repo identity. git-remote-aspen detects this and routes through the local cluster's federation layer instead of direct Forge RPC.
- **New RPC**: `FederationGitClone` / `FederationGitFetch` — the local cluster receives the git-remote-helper's list/fetch requests and proxies them through the existing federation sync protocol to the origin cluster.
- **On-demand mirror creation**: When Bob's cluster receives a federated clone request for a repo it hasn't seen, it creates a mirror repo, pulls refs + objects from Alice via `sync_objects`, imports them, and serves them back to the git client.
- **Subsequent fetches use pull**: After the initial clone, `git fetch` hits the mirror. If the mirror is stale, the cluster does an incremental federation pull (using `have_hashes`) before responding.
- **NixOS VM integration test**: Two-cluster test where Bob clones Alice's repo via federated URL and verifies the working tree matches.

## Capabilities

### New Capabilities

- `federated-git-clone`: End-to-end `git clone aspen://` for repos hosted on remote federated clusters, including URL parsing, RPC proxying, on-demand mirror creation, and incremental fetch.

### Modified Capabilities

## Impact

- `src/bin/git-remote-aspen/` — URL parsing (new `FederatedId` variant), fetch/list routing
- `crates/aspen-client-api/` — new RPC request/response variants
- `crates/aspen-rpc-handlers/` or `crates/aspen-forge-handler/` — handler for federated git requests
- `crates/aspen-forge/` — mirror creation from federation sync objects
- `nix/tests/` — new two-cluster VM test
