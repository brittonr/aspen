## Why

Molten's runtime spine and choreography layer define how actors exchange admitted envelopes, but some runtime state must be strongly consistent across a group of nodes. Examples include installed protocol registries, membership, capability/grant state, durable receipt indexes, replay ledgers, and linearizable configuration or policy updates. These should not be modeled as ordinary best-effort dataspace assertions or gossip messages.

Trellis already provides verified Raft-oriented primitives for elections, quorums, log matching, append entries, commit advancement, read-index/lease-read admission, membership changes, snapshots, client-session deduplication, and linearizability/state-machine safety. Molten should use those primitives as the consensus specification/admission layer for replicated control-plane state.

## What Changes

- Define a Molten consensus surface for strongly consistent control-plane state.
- Add a Raft group manifest that identifies group id, members, timeouts, read mode, snapshot policy, state-machine kind, and policy references.
- Represent replicated commands as canonical Molten command envelopes with stable hashes, client session ids, sequence numbers, capabilities, and receipt/evidence references.
- Keep the replicated state machine deterministic and side-effect free; adapters persist logs/snapshots and emit effects only after admission.
- Use Trellis Raft primitives as the normative bounded admission/spec layer for log, quorum, membership, read, snapshot, and client-session behavior.
- Route Raft messages over the existing envelope spine so local dataspace and Iroh transport can carry them without changing consensus semantics.
- Gate membership changes, command proposals, snapshots, and state-machine reads through Basalt/Nickel/Steel/Trellis/Cairn policy and receipt boundaries.
- Limit the first consensus scope to control-plane state; normal actor messages, choreography step traffic, gossip, blob transfer, and local dataspace assertions do not require Raft.

## Impact

This creates a future `molten-consensus` contract that can be implemented incrementally after the runtime spine exists. The Cairn does not require a complete Raft engine immediately; it defines the boundaries, artifacts, state-machine expectations, policy hooks, and test obligations for any implementation that claims to provide Molten's strongly consistent control plane.
