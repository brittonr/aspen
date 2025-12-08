# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Aspen is a foundational orchestration layer for distributed systems, written in Rust. It provides distributed primitives for managing and coordinating distributed systems within the Blixard ecosystem, drawing inspiration from Erlang/BEAM, Plan9, Kubernetes, FoundationDB, etcd, and Antithesis.

The codebase recently underwent a major refactoring to decouple and restructure the architecture. The current implementation is deliberately minimal, exposing lightweight scaffolding through narrow trait-based APIs while the real implementations are being wired back in.

## Core Technologies

- **openraft**: Raft consensus algorithm for cluster-wide linearizability and fault-tolerant replication (vendored)
- **redb**: Embedded ACID storage engine for Raft log storage (append-only, fast sequential writes)
- **rusqlite**: SQLite-based state machine storage (ACID transactions, queryable)
- **iroh**: Peer-to-peer networking and content-addressed communication
- **ractor/ractor_cluster**: Actor-based concurrency and distributed computing primitives
- **hiqlite**: SQLite-based distributed database with cache and distributed lock features
- **madsim**: Deterministic simulator for distributed systems testing
- **snafu/anyhow**: Error handling (snafu for library errors, anyhow for application errors)
- **proptest**: Property-based testing

## Vendored Dependencies

### openraft (Vendored at `openraft/openraft`)

**Rationale**: Aspen vendors openraft v0.10.0 to maintain tight control over the consensus layer, enabling:

1. **Local modifications without upstream delays**: Foundational distributed systems require rapid iteration on core primitives. Vendoring allows immediate fixes and enhancements without waiting for upstream review/release cycles.

2. **API stability during development**: Aspen is in active development with frequent architectural changes. Vendoring prevents breaking changes from upstream openraft releases from disrupting iteration.

3. **Custom optimizations**: Enables performance tuning specific to Aspen's workload patterns (e.g., append-optimized log storage with redb, SQLite state machine integration).

4. **Dependency isolation**: Prevents supply chain issues from upstream dependency changes that might conflict with Aspen's other dependencies (madsim, ractor, iroh).

**Current Status**:

- Vendored version: openraft 0.10.0
- Upstream crates.io fallback: 0.9.21 (configured in deny.toml to skip duplicate checking)
- Local modifications: Tracked via git history in `openraft/` directory

**Update Procedure**:

1. Review upstream openraft releases for relevant changes
2. Evaluate changes for compatibility with Aspen's architecture
3. Cherry-pick or rebase local modifications onto new upstream version
4. Update Cargo.toml path dependency version if needed
5. Run full test suite (especially madsim deterministic tests) to verify compatibility
6. Document any new local modifications in commit messages

**Long-term Strategy**: As Aspen matures and the API stabilizes, evaluate transitioning back to crates.io version if local modifications can be upstreamed or if they're no longer necessary.

## Architecture

The project is structured into focused modules with narrow APIs:

- **src/api/**: Trait definitions for `ClusterController` and `KeyValueStore` interfaces
  - `ClusterController`: Manages cluster membership (init, add learner, change membership)
  - `KeyValueStore`: Handles distributed key-value operations (read, write)
  - Contains `DeterministicClusterController` and `DeterministicKeyValueStore` in-memory implementations for testing
- **src/raft/**: Raft-based consensus implementation
  - `RaftActor`: Actor that drives the Raft state machine using ractor
  - `RaftControlClient`: Controller that proxies operations through the Raft actor
  - `storage.rs`: Log storage backed by redb, state machine storage backed by SQLite
  - `storage_sqlite.rs`: SQLite state machine implementation (default, production-ready)
  - `network.rs`: Network layer for Raft RPC
  - `types.rs`: Type configurations for openraft
- **src/kv/**: Key-value service builder and node management
  - `KvServiceBuilder`: Deterministic builder for KV service configuration
  - Generates Iroh endpoint IDs from node IDs for testing
- **src/cluster/**: Cluster coordination and transport
  - `NodeServerHandle`: Manages ractor_cluster node server lifecycle
  - `IrohEndpointManager`: Manages Iroh P2P endpoint and discovery services
  - `bootstrap.rs`: Node bootstrap orchestration
  - `config.rs`: Cluster configuration types
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
- Edition is set to `2024` in Cargo.toml (Rust 2024 edition)
