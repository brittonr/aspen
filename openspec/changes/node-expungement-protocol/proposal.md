## Why

When a node is removed from Raft membership, Aspen treats it as a routine voter removal. The node's Iroh key is still trusted if it reconnects, and there's no persistent record that the node was deliberately excluded. Oxide's trust-quorum has an explicit expungement protocol: removed nodes receive an `Expunged(epoch)` message, record it permanently, and refuse all further trust operations. This prevents removed nodes from participating in share collection or reconstructing secrets. Without expungement, a removed node that comes back online could still respond to `GetShare` requests and leak old shares.

## What Changes

- Add `Expunged` message type to the trust protocol (sent when a node is removed from configuration)
- Add persistent `ExpungedMetadata { epoch, removed_by }` to node state — once set, permanent
- Expunged nodes reject all trust protocol messages (GetShare, Prepare, etc.)
- Expunged nodes reject all Raft operations (prevent zombie rejoining)
- Provide `aspen-cli cluster expunge <node-id>` command for operator-initiated removal
- Require factory reset (wipe data_dir) for an expunged node to be re-added

## Capabilities

### New Capabilities

- `node-expungement`: Permanent node removal from the trust domain with persistent expungement marker and protocol-level rejection of further operations

### Modified Capabilities

## Impact

- `crates/aspen-trust/` — expungement message handling, persistent state marker
- `crates/aspen-raft/` — expunged nodes reject Raft RPCs
- `crates/aspen-cluster/` — expunged nodes refuse Iroh connections from cluster peers
- `crates/aspen-cli/` — `cluster expunge` command
- `crates/aspen-core/` — `ExpungedMetadata` type
