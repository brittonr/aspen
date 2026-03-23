## ADDED Requirements

### Requirement: CLI update-peer subcommand

The CLI provides a `cluster update-peer` subcommand that sends `ClientRpcRequest::AddPeer` to a target node, updating its Raft network factory peer address map without Raft consensus.

#### Scenario: Update a follower's peer address for the leader

- **WHEN** `aspen-cli --ticket <follower-ticket> cluster update-peer --node-id 1 --addr '{"id":"...","addrs":[{"Ip":"host:port"}]}'` is executed
- **THEN** the follower's network factory `peer_addrs` map is updated with the new address for node 1, and any stale connection for node 1 is evicted from the connection pool

#### Scenario: Invalid address JSON

- **WHEN** `cluster update-peer --addr 'not-json'` is executed
- **THEN** the CLI prints an error from the server indicating invalid endpoint_addr format

#### Scenario: JSON output mode

- **WHEN** `cluster update-peer --json` is used
- **THEN** the response includes `is_success` and optional `error` fields

### Requirement: Dogfood deploy address refresh

After a rolling deploy restarts the leader node, the dogfood script pushes the leader's new address to each follower using `cluster update-peer`, restoring Raft connectivity without gossip.

#### Scenario: 3-node rolling deploy completes with all nodes connected

- **WHEN** a 3-node rolling deploy restarts followers (2, 3) then leader (1)
- **THEN** the dogfood script calls `update-peer` on each follower with the leader's new address, and the post-deploy `assert_cluster_healthy` passes (leader agreement + KV round-trip)

#### Scenario: Full address sweep after all restarts

- **WHEN** all nodes have been restarted
- **THEN** the dogfood script runs a full address sweep: for each node, tells every other node about its current address via `update-peer`
