# 10. Madsim Deterministic Simulation Testing

**Status:** accepted

## Context

Distributed systems have failure modes that are hard to reproduce: network partitions, message reordering, clock skew, and crash-recovery races. Traditional integration tests run against real networks and real clocks, making these failures non-deterministic — a test may pass 999 times and fail on the 1000th run.

Madsim is a deterministic simulation framework for Rust async programs. It replaces tokio's runtime with a simulated runtime where time, network, and randomness are controlled by a seed. Given the same seed, the same execution happens every time. This is the same approach used by FoundationDB and TigerBeetle.

## Decision

Use madsim for deterministic simulation tests of Aspen's distributed protocols. The `aspen-testing-madsim` crate provides test utilities, and the `simulation` feature flag swaps real I/O for madsim's simulated I/O.

Madsim tests can:

- Inject network partitions between specific nodes
- Control message delivery order and timing
- Simulate clock skew
- Reproduce any failure by re-running with the same seed

The FCIS pattern (ADR-004) is a prerequisite — deterministic simulation only works when business logic has no hidden dependencies on real time or real I/O.

Test profiles:

- `default`: Standard tests with 60s timeout
- `quick`: Skips slow tests including madsim multi-node scenarios
- `ci`: Extended 120s timeouts for simulation tests

Alternatives considered:

- (+) Jepsen-style testing: proven for finding distributed bugs (used by CockroachDB, etc.)
- (-) Jepsen: requires real cluster infrastructure, non-deterministic, slow, Clojure tooling
- (+) Property-based testing (proptest/Bolero): finds edge cases via random generation
- (~) Property-based: good for pure functions but doesn't simulate network failures
- (+) Madsim: deterministic, reproducible, fast, same-process simulation
- (-) Madsim: requires code to be simulation-compatible (no raw I/O in business logic)
- (+) Antithesis: commercial deterministic simulation platform
- (-) Antithesis: external service, cost, less control over simulation parameters

## Consequences

- Distributed protocol bugs are reproducible — re-run with the same seed to get the same failure
- Tests run in simulated time — a "30-minute timeout" test completes in milliseconds
- Network partition tests are deterministic, not flaky
- All business logic must follow FCIS (ADR-004) to be simulation-compatible
- The `simulation` feature flag adds conditional compilation — code must work in both real and simulated modes
- Madsim tests complement (not replace) real integration tests — the NixOS VM tests exercise real networking
