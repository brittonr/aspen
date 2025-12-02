# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Aspen is a foundational orchestration layer for distributed systems, written in Rust. It provides distributed primitives for managing and coordinating distributed systems within the Blixard ecosystem, drawing inspiration from Erlang/BEAM, Plan9, Kubernetes, FoundationDB, etcd, and Antithesis.

The codebase recently underwent a major refactoring to decouple and restructure the architecture. The current implementation is deliberately minimal, exposing lightweight scaffolding through narrow trait-based APIs while the real implementations are being wired back in.

## Core Technologies

- **openraft**: Raft consensus algorithm for cluster-wide linearizability and fault-tolerant replication
- **redb**: Embedded ACID storage engine powering each node's Raft state machine
- **iroh**: Peer-to-peer networking and content-addressed communication
- **ractor/ractor_cluster**: Actor-based concurrency and distributed computing primitives
- **kameo**: Alternative actor framework with remote capabilities (experimental)
- **hiqlite**: SQLite-based distributed database with cache and distributed lock features
- **madsim**: Deterministic simulator for distributed systems testing
- **snafu/anyhow**: Error handling (snafu for library errors, anyhow for application errors)
- **proptest**: Property-based testing

## Architecture

The project is structured into focused modules with narrow APIs:

- **src/api/**: Trait definitions for `ClusterController` and `KeyValueStore` interfaces
  - `ClusterController`: Manages cluster membership (init, add learner, change membership)
  - `KeyValueStore`: Handles distributed key-value operations (read, write)
  - Contains `DeterministicClusterController` and `DeterministicKeyValueStore` in-memory implementations for testing
- **src/raft/**: Raft-based consensus implementation
  - `RaftActor`: Actor that drives the Raft state machine using ractor
  - `RaftControlClient`: Controller that proxies operations through the Raft actor
  - `storage.rs`: State machine store backed by redb
  - `network.rs`: Network layer for Raft RPC
  - `types.rs`: Type configurations for openraft
- **src/kv/**: Key-value service builder and node management
  - `KvServiceBuilder`: Deterministic builder for KV service configuration
  - Generates Iroh endpoint IDs from node IDs for testing
- **src/cluster/**: Cluster coordination and transport
  - `NodeServerHandle`: Manages ractor_cluster node server lifecycle
  - `IrohClusterTransport`: Simulated Iroh-backed transport for testing
  - `DeterministicClusterConfig`: Configuration for deterministic simulations
- **src/main.rs**: Basic Kameo-based remote actor example (experimental)
- **src/bin/aspen-node.rs**: Node binary for cluster deployments

## Development Commands

### Building and Testing

```bash
# Build the project (inside Nix environment)
nix develop -c cargo build

# Run tests with cargo-nextest
nix develop -c cargo nextest run

# Run a specific test
nix develop -c cargo nextest run <test_name>

# Format code with Nix formatter
nix fmt

# Run clippy lints
nix develop -c cargo clippy --all-targets -- --deny warnings
```

### Running Scripts

```bash
# Run Aspen cluster smoke test (3-node cluster with Raft operations)
./scripts/aspen-cluster-smoke.sh

# Run Hiqlite cluster smoke test
./scripts/aspen-hiqlite-smoke.sh

# Launch Hiqlite cluster manually
./scripts/run-hiqlite-cluster.sh
```

### Nix Development

```bash
# Enter the Nix development shell
nix develop

# Run commands in Nix environment without entering shell
nix develop -c <command>

# Check flake
nix flake check

# Build specific outputs
nix build .#<output>
```

## Coding Style: Tiger Style

Aspen follows "Tiger Style" principles (see tigerstyle.md):

### Safety Principles
- Use explicitly sized types (`u32`, `i64`) instead of `usize` for portability
- Set fixed limits on loops, queues, and data structures to prevent unbounded resource use
- Static memory allocation preferred; avoid dynamic allocation after initialization
- Minimize variable scope; declare variables close to their usage
- Use assertions to verify invariants and function pre/post-conditions
- Fail fast on programmer errors; handle all errors explicitly
- Centralize control flow in parent functions; move non-branching logic to helper functions
- Keep functions under 70 lines

### Performance Principles
- Design for performance early; use "napkin math" for quick estimates
- Optimize resources in order: network → disk → memory → CPU
- Batch operations to amortize expensive operations
- Write predictable code paths to optimize CPU cache and branch prediction

### Developer Experience Principles
- Clear and consistent naming: use `snake_case`, avoid abbreviations
- Include units or qualifiers in variable names in descending order (`latency_ms_max`)
- Document the "why" with comments, not just the "what"
- Organize code logically: high-level abstractions before low-level details
- Simplify function signatures; limit parameters and prefer simple return types
- Avoid duplicates and aliases; maintain single source of truth
- Pass large objects (>16 bytes) by reference

### Zero Technical Debt
- Do it right the first time; avoid rushing features that create debt
- Be proactive in problem-solving; anticipate and fix issues early

## Testing Philosophy

- **Integration over unit testing**: Currently prioritizing integration tests with real services over unit tests
- **Deterministic simulation**: Use `madsim` for deterministic distributed system testing
  - Simulation artifacts are automatically captured and persisted to `docs/simulations/`
  - Each artifact includes: seed, event trace, metrics snapshot, duration, and status
  - Use `SimulationArtifactBuilder` from `src/simulation.rs` to instrument new simulation tests
  - Artifacts are gitignored but collected by CI for failed test debugging
- **Property-based testing**: Use `proptest` for exploring edge cases
- **Test modifications**: Never modify, remove, or add tests unless explicitly asked

## Important Notes

- The codebase previously had richer integration between openraft, Iroh, and ractor. Those modules were removed for a deliberate rebuild with cleaner boundaries.
- Keep Iroh P2P and Hiqlite database coupling as-is; these are core infrastructure services
- Backwards compatibility is not a concern; prioritize clean, modern solutions
- The project contains a vendored/embedded `openraft` directory with local modifications
- A separate `kameo` directory exists for actor framework experiments
- Edition is set to `2024` in Cargo.toml (Rust 2024 edition)
