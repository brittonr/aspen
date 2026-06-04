## Why

Molten needs strongly consistent shared state for protocol/artifact/policy registries, receipt indexes, and coordination services, but normal actor traffic must stay out of Raft. A first bounded Trellis-backed Raft state machine gives Molten a control-plane registry without turning the dataspace into a global log.

## What Changes

- Add canonical Raft group manifest, command envelope, log entry, snapshot, read-index, and registry apply receipt records.
- Implement a first replicated control-plane state machine for installed protocol/artifact/policy registry pointers.
- Wrap Trellis Raft predicates for append consistency, quorum/commit, read-index, client-session idempotency, and snapshot integrity.
- Persist logs/snapshots through local Redb/chunk evidence and emit receipts for proposal, commit, apply, read, and recovery.
- Explicitly reject ordinary actor messages, gossip fanout, and blob transfer as Raft commands.

## Impact

This creates the first strongly consistent Molten control-plane path and a reusable substrate for coordination primitives, without changing ordinary SAM/dataspace message semantics.
