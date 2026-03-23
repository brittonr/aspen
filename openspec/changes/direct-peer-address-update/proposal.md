## Why

After a rolling deploy restarts all nodes in a 3-node cluster, Raft followers lose connectivity to the leader because the leader's iroh port changed on restart. Gossip can't re-bootstrap (all bootstrap addresses are stale), and `add-learner` only updates the leader's Raft membership — followers never learn the leader's new address in their network factory. The cluster stays partitioned until manual intervention. This blocks the dogfood full-loop pipeline.

## What Changes

- Add `cluster update-peer` CLI subcommand that sends the existing `ClientRpcRequest::AddPeer` RPC (handler already wired at `aspen-cluster-handler`, calls `factory.add_peer()` with connection pool eviction — no Raft consensus needed)
- Update the dogfood deploy script to call `update-peer` on each follower after the leader restarts, pushing the leader's new address directly to their network factories

## Capabilities

### New Capabilities

- `peer-address-update`: CLI subcommand for the existing direct peer address update RPC, with dogfood deploy integration

### Modified Capabilities

## Impact

- `crates/aspen-cli/src/bin/aspen-cli/commands/cluster.rs`: New `UpdatePeer` subcommand (args: `--node-id`, `--addr`)
- `crates/aspen-cli/src/bin/aspen-cli/output.rs`: Output type for the response
- `scripts/dogfood-local.sh`: Call `update-peer` on each follower after leader restart
