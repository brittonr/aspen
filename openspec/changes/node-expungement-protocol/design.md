## Context

Aspen removes voters via openraft's `change_membership`. This is a consensus-level operation — the node is no longer a voter. But there's no security-level exclusion. The removed node still has a valid Iroh key, valid Raft state, and its trust share. It could reconnect and participate in protocols it shouldn't.

Oxide's trust-quorum treats expungement as a permanent, persistent state change. Once a node is expunged, it sets a flag that survives reboots and causes all protocol messages to be dropped. The node must be factory reset (full data wipe) before it can rejoin a cluster.

## Goals / Non-Goals

**Goals:**

- Define expungement as a permanent, persistent marker on a node
- Deliver `Expunged(epoch)` notification to removed nodes
- Expunged nodes reject all trust protocol messages (share requests, prepare, etc.)
- Expunged nodes reject Raft RPCs to prevent zombie participation
- Zeroize the node's trust share on expungement (defense in depth)
- Provide CLI command and programmatic API for operator-initiated expungement
- Document recovery path: factory reset required to re-add

**Non-Goals:**

- Automatic expungement on network partition (only explicit operator or Raft leader action)
- Revoking the node's Iroh key (would require a key revocation list — future work)
- Distributed consensus on expungement (the Raft membership change IS the consensus)

## Decisions

**Expungement is a side effect of membership removal + trust reconfiguration.** When a node is removed from the trust configuration (via `secret-rotation-on-membership-change`), the leader sends `Expunged(new_epoch)` to the removed node. The removed node records `ExpungedMetadata { epoch, removed_by }` and enters a terminal state.

**The expunged marker is checked first in every handler.** Every trust protocol message handler and Raft RPC handler checks `is_expunged()` before processing. This is a single `if` check and keeps the security boundary at the top of every code path.

**Share zeroization on expungement.** When a node marks itself as expunged, it immediately zeroizes its share(s) in redb by overwriting with zeros, then deleting. This limits the window where a compromised expunged disk could leak share material.

**Peers also enforce expungement.** When a non-expunged node receives a `GetShare` request from a node not in the current configuration, it responds with `Expunged(current_epoch)` instead of the share. This handles the case where the removed node didn't receive the initial expungement message.

**No automatic re-addition.** An expunged node cannot be re-added by simply calling `add_learner`. The `data_dir` must be wiped first (removing the expunged marker). The node then bootstraps as a completely new member with a fresh Iroh key.

## Risks / Trade-offs

- **Operator error**: Accidental expungement requires factory reset. Mitigation: CLI requires `--confirm` flag and prints warnings.
- **Message delivery**: If the `Expunged` message is lost, the node doesn't know it's been removed. Mitigation: peers enforce expungement on any incoming request from the removed node.
- **Race window**: Between Raft membership removal and the node receiving its `Expunged` message, the node might respond to share requests. Mitigation: the new configuration's share digests are different, so old shares won't validate against the new config.
