## Context

Aspen's formal verification story has two layers: Verus for function-level proofs (deterministic, no I/O) and madsim for distributed simulation testing (randomized, specific scenarios). The gap is exhaustive protocol-level model checking — exploring ALL possible interleavings of messages, crashes, and recoveries.

Oxide's trust-quorum TLA+ spec (`tla/trust_quorum.tla`) is ~600 lines and models 5 nodes, configurable epochs, and crash/restart behavior. It checks type safety, config consistency, and that committed nodes always have shares. Their spec uses several state-space reduction techniques (CHOOSE over \E for message receipt, fixed configuration sequences, crash limits) that keep TLC tractable.

## Goals / Non-Goals

**Goals:**

- Write TLA+ specs for Aspen's trust protocol: cluster secret init, share distribution, reconfiguration, expungement
- Define safety invariants that must hold in ALL reachable states
- Define liveness properties that must eventually hold under fairness
- Integrate TLC into CI so model checking runs on every change
- Keep specs tractable: 3-5 nodes, bounded epochs, bounded crashes
- Document the correspondence between TLA+ spec and Rust code

**Non-Goals:**

- Specifying the full Raft protocol in TLA+ (already done by Diego Ongaro's original TLA+ spec; openraft's correctness is trusted)
- Specifying blob transfer, gossip, or other non-trust protocols (future work)
- TLAPS proofs (TLC model checking is sufficient for now)
- Replacing madsim (TLA+ checks the protocol design; madsim checks the implementation)

## Decisions

**Separate specs per concern.** Rather than one monolithic spec, use separate TLA+ modules:

- `trust_init.tla` — cluster secret creation and initial share distribution
- `trust_reconfig.tla` — reconfiguration with share collection and secret rotation
- `trust_expunge.tla` — node expungement and peer enforcement
- `trust_combined.tla` — composition of all three for full protocol checking

**Raft as an oracle.** The TLA+ spec does not model Raft internals. It models Raft's guarantees: linearizable writes, leader election, membership change. The Raft leader is treated as an oracle that can propose and commit entries. This matches Oxide's approach (Nexus as oracle) adapted for Aspen's Raft-leader-as-coordinator design.

**Global message set (not channels).** Following Oxide's pattern: all messages ever sent are kept in a global set. Receipt is gated by validity predicates. This is simpler than per-channel queues and avoids modeling message loss separately (a message is "lost" if its validity predicate is never satisfied). State-space reduction via CHOOSE instead of \E.

**Fixed configuration sequences.** Rather than generating arbitrary configurations (which explodes the state space), use a fixed sequence of 3 configurations. This tests the transitions that matter while keeping TLC tractable.

**Nix integration.** Package TLC as a Nix derivation. `nix run .#check-tla` runs all specs. `nix flake check` includes TLA+ model checking alongside Rust tests. The TLC runner is hermetic — no Java installation required on the dev machine.

## Risks / Trade-offs

- **Learning curve.** TLA+ is unfamiliar to most Rust developers. Mitigation: document conventions thoroughly, keep specs small and commented, and pair TLA+ invariants with corresponding Rust assertions.
- **State space explosion.** Model checking is exponential in the number of nodes and epochs. Mitigation: start with 3 nodes and 2 epochs, increase gradually. Use symmetry sets and CHOOSE where possible.
- **Spec-code drift.** TLA+ specs can diverge from Rust implementation over time. Mitigation: document the mapping (spec variable → Rust field), add comments in Rust code referencing TLA+ invariants, review specs when protocol code changes.
- **CI time.** TLC can take minutes for large state spaces. Mitigation: use small models in CI (3 nodes, 2 epochs), larger models for periodic deep checks.
