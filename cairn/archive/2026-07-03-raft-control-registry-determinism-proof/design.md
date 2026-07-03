# Design: Raft control-registry determinism proof

## Scope

This change proves the deterministic replicated state-machine boundary for the control registry. It covers admitted command envelopes, log append/commit receipts, duplicate client-session handling, read-index evidence, snapshots, and restore receipts.

## Proof checklist

- **Proof claim**: two control-registry runtimes applying the same admitted command log produce the same registry state ref, receipt refs, and read results; duplicates do not advance state twice; snapshots restore equivalent state.
- **Out of scope**: Raft leader election timing, network partitions, disk durability, and transport-level delivery.
- **Trusted assumptions**: command envelope canonical hashing and predicate receipt hashing remain stable.
- **Positive evidence**: generated admitted command logs converge across independent runtimes and snapshot/restore preserves state refs.
- **Negative evidence**: duplicate client sequence, unsupported state machine id, malformed command schema, stale read-index, and tampered snapshot evidence deny or replay without mutation.
- **Canonical refs**: proof traces bind command envelope refs, log entry refs, commit receipt refs, registry receipt refs, state refs, read receipts, and snapshot refs.
- **Regeneration command**: `cargo test raft`.

## Functional core

The deterministic apply logic should remain side-effect free over in-memory state. Persistence, filesystem snapshots, and CLI rendering should remain shell concerns around canonical state values.

## Non-goals

- No proof of distributed consensus liveness.
- No claim that storage adapters cannot lose bytes outside their own evidence gates.
