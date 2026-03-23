## Context

Rolling deploys restart nodes one at a time. Each restart gives the node a new iroh QUIC port. The dogfood script restarts followers first, then the leader. After the leader restarts, followers still have its old address in their Raft network factory's `peer_addrs` map. Raft heartbeats route to the dead port, followers stay as Candidates forever.

The `AddPeer` RPC already exists and is fully wired:

- `ClientRpcRequest::AddPeer { node_id, endpoint_addr }` in `aspen-client-api`
- Handler in `aspen-cluster-handler` calls `factory.add_peer()` which updates `peer_addrs` and evicts stale connections from the pool
- No Raft consensus — operates directly on the local node's network factory

What's missing: a CLI subcommand to invoke it, and dogfood script integration.

## Goals / Non-Goals

**Goals:**

- Add `cluster update-peer` CLI subcommand that calls `ClientRpcRequest::AddPeer`
- Update dogfood deploy to push the leader's new address to each follower after leader restart
- Unblock the 3-node rolling deploy full-loop

**Non-Goals:**

- Fixing the root cause in gossip bootstrap (gossip should recover from all-stale-addresses, but that's a separate issue)
- Adding automatic peer address propagation in the Raft layer
- Changing the AddPeer RPC handler (it already works correctly)

## Approach

### 1. CLI subcommand

Add `cluster update-peer` to `ClusterCommand` enum in `crates/aspen-cli/src/bin/aspen-cli/commands/cluster.rs`. Same arg pattern as `add-learner` (`--node-id`, `--addr`). Sends `ClientRpcRequest::AddPeer`, prints `AddPeerResultResponse`.

### 2. Dogfood script integration

After the leader restarts in `do_deploy()`, extract the leader's new endpoint ID and address from its log, then for each follower call:

```bash
cli_node "$follower" cluster update-peer --node-id "$leader_id" \
  --addr '{"id":"...","addrs":[{"Ip":"host:port"}]}'
```

This bypasses Raft entirely — each follower updates its own factory. The leader already has follower addresses from the per-node `add-learner` calls during the earlier restarts.

### 3. Also announce follower addresses to each other

After the leader restart, the leader's factory has stale addresses for followers too (the factory was rebuilt from the peer cache, which has pre-restart ports). The leader got updated via `add-learner` during the deploy loop, but other followers may not have each other's new addresses. Run a full address sweep: for each node, tell all other nodes about its current address.

## Key Details

- `update-peer` is a local operation on each node — no quorum needed, no leader required
- The handler already parses JSON `EndpointAddr` and calls `add_peer()` with eviction
- Existing tests in `aspen-cluster-handler` cover the handler (`test_handle_add_peer_*`)
- No new dependencies or feature flags needed

## Alternatives Considered

1. **Fix gossip bootstrap** — gossip should eventually propagate new addresses, but it requires at least one valid bootstrap peer. After all nodes restart, all bootstrap addresses are stale. This is the right long-term fix but requires changes to the gossip subsystem.
2. **Leader transfer before leader restart** — transfer leadership to a follower (already on new binary) before restarting the old leader. Avoids the problem but adds complexity to the deploy sequence.
3. **Shared address file** — write each node's address to a file after restart, have other nodes read it. Works but fragile and non-standard.
