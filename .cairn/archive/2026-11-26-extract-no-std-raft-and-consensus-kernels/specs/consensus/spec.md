# Consensus deltas

## ADDED Requirements

### Requirement: Replicated Raft transitions and reliability workload models execute as no_std kernels
r[molten.consensus.no_std_raft_and_reliability_kernels] Replicated Raft state transitions (`molten-raft-core`), fast-path transitions (`molten-fastpath-core`, if retained), and reliability workload/history/fault-profile models (`molten-reliability-model-core`) MUST live in `#![no_std]` + `alloc` crates with zero Tokio, Iroh, Redb, wall-clock, or `preserves::IOValue` dependencies, invoked by thin shells in `molten`.

#### Scenario: Raft transitions execute deterministically without std
- GIVEN serialized replica state and a sequence of election, replication, read-probe, snapshot, and rejoin events
- WHEN `molten-raft-core` processes the events under `#![no_std]`
- THEN identical inputs MUST emit identical next states and typed effect plans without OS clocks, randomness, or sockets.

#### Scenario: Impure I/O import cannot enter molten-raft-core
- GIVEN an attempt to import `tokio`, `iroh`, `redb`, or `std::time` inside `crates/molten-raft-core`
- WHEN the hermetic `no_std` CI check runs
- THEN compilation MUST fail closed.
