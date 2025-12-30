# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Aspen is a foundational orchestration layer for distributed systems, written in Rust. It provides distributed primitives for managing and coordinating distributed systems within the Blixard ecosystem, drawing inspiration from Erlang/BEAM, Plan9, Kubernetes, FoundationDB, etcd, and Antithesis.

The codebase recently underwent a major refactoring (completed Dec 13, 2025) to remove the actor-based architecture in favor of direct async APIs. The current implementation is **fully functional and production-ready** with approximately 21,000+ lines of code and 350+ passing tests. All trait-based APIs have complete implementations with no stubs or placeholders.

## Core Technologies

- **openraft**: Raft consensus algorithm for cluster-wide linearizability and fault-tolerant replication (vendored)
- **redb**: Embedded ACID storage engine for unified Raft log + state machine storage (single-fsync writes, ~2-3ms latency)
- **DataFusion**: Apache DataFusion SQL query engine over Redb KV data (optional, via `sql` feature)
- **iroh**: Peer-to-peer networking and content-addressed communication (QUIC, NAT traversal, discovery)
- **iroh-blobs**: Content-addressed blob storage for large values
- **iroh-docs**: CRDT-based document synchronization for real-time KV replication
- **mainline**: BitTorrent Mainline DHT for global content discovery (optional, via `global-discovery` feature)
- **hickory-server**: DNS protocol server for DNS record management (optional, via `dns` feature)
- **fuse-backend-rs**: FUSE filesystem support for mounting Aspen as a filesystem (optional, via `fuse` feature)
- **madsim**: Deterministic simulator for distributed systems testing
- **snafu/anyhow**: Error handling (snafu for library errors, anyhow for application errors)
- **proptest**: Property-based testing
- **bolero**: Unified testing framework combining fuzz testing and property-based testing (same tests run as fuzzing with libFuzzer/AFL or as property tests in CI)
- **ratatui**: Terminal UI framework (optional, via `tui` feature)

## Vendored Dependencies

### openraft (Vendored at `openraft/openraft`)

**Rationale**: Aspen vendors openraft v0.10.0 to maintain tight control over the consensus layer, enabling:

1. **Local modifications without upstream delays**: Foundational distributed systems require rapid iteration on core primitives. Vendoring allows immediate fixes and enhancements without waiting for upstream review/release cycles.

2. **API stability during development**: Aspen is in active development with frequent architectural changes. Vendoring prevents breaking changes from upstream openraft releases from disrupting iteration.

3. **Custom optimizations**: Enables performance tuning specific to Aspen's workload patterns (e.g., single-fsync unified log+state machine with redb).

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

## Distributed Cluster Architecture Patterns

Aspen is a distributed cluster system built on direct async APIs with clean separation between consensus, networking, and storage layers.

### Core Distributed Components (All Required)

1. **Raft (via openraft)**: Ensures cluster-wide consensus and linearizable operations
2. **Iroh P2P**: Handles all inter-node networking, discovery, and NAT traversal
3. **Storage (redb)**: Unified log + state machine storage with single-fsync writes

These components work together through direct async interfaces without actor indirection.

### Current Architecture

```
Client RPC (Iroh QUIC) / Terminal UI (ratatui)
         ↓
ClusterController + KeyValueStore Traits
         ↓
RaftNode (Direct Async Implementation)
         ↓
OpenRaft 0.10.0 (vendored)
    ├── SharedRedbStorage (unified log + state machine, single-fsync)
    ├── DataFusion SQL (SQL queries over Redb KV)
    └── IrpcRaftNetwork (IRPC over Iroh)
         ↓
IrohEndpointManager (QUIC + NAT traversal)
    ├── mDNS discovery (local networks)
    ├── DNS discovery (production)
    ├── Pkarr (DHT fallback)
    ├── Gossip (peer announcements)
    ├── Mainline DHT (global content discovery, optional with `global-discovery` feature)
    │   └── BEP-44 mutable items for cross-cluster discovery
    ├── iroh-blobs (content-addressed storage)
    └── iroh-docs (CRDT replication)
```

### Implementation Guidelines

**API Design: Iroh-First, No HTTP**

Aspen uses Iroh for ALL client and inter-node communication. Do NOT add HTTP endpoints or use axum for new features. The existing HTTP API in `aspen-node.rs` is legacy and will be removed.

- **Client APIs**: Use Iroh Client RPC protocol (`CLIENT_ALPN`) via `ClientProtocolHandler`
- **Node-to-node**: Use Iroh with ALPN-based protocol routing
- **Blob transfer**: Use iroh-blobs protocol
- **Real-time sync**: Use iroh-docs for CRDT-based replication

**For new distributed features:**

1. Use Iroh for ALL network communication (no raw TCP/HTTP)
2. Go through Raft consensus for any state that needs cluster-wide agreement
3. Use the trait-based API (`ClusterController` and `KeyValueStore`) for abstraction
4. Follow Tiger Style resource bounds (see constants.rs for limits)

**Networking patterns:**

- One Iroh endpoint per node (managed by `IrohEndpointManager`)
- Use ALPN-based protocol routing (RAFT_ALPN, TUI_ALPN, GOSSIP_ALPN)
- Connection pooling for QUIC stream reuse
- Gossip for peer discovery, NOT for Raft membership (security separation)

**Storage patterns:**

- SharedRedbStorage for unified log + state machine (single-fsync, ~2-3ms latency)
- DataFusion SQL for queryable access to KV data
- Batch operations limited to prevent unbounded memory use

**Never:**

- Add HTTP endpoints or use axum for new APIs (use Iroh Client RPC instead)
- Implement node-to-node communication without Iroh
- Create distributed state without Raft consensus
- Allow unbounded operations (always use Tiger Style limits)
- Mix transport layer (Iroh) with application layer (Raft membership)

## Architecture

See `docs/architecture/` for detailed architecture documents:

- `layer-architecture.md` - Analysis of FoundationDB-style layer patterns (tuples, subspaces, indexes)
- `migration.md` - Migration plan history (SQLite removal complete as of Dec 26, 2025)

The project is structured into focused modules with narrow APIs:

- **src/api/** (393 lines): Trait definitions for `ClusterController` and `KeyValueStore` interfaces
  - `ClusterController`: Manages cluster membership (init, add learner, change membership, get metrics, trigger snapshot)
  - `KeyValueStore`: Handles distributed key-value operations (read, write, delete, scan with pagination)
  - Contains `DeterministicClusterController` and `DeterministicKeyValueStore` in-memory implementations for testing
- **src/raft/** (~6,500 lines): Raft-based consensus implementation
  - `node.rs`: RaftNode that implements both ClusterController and KeyValueStore traits
  - `storage.rs`: In-memory and standalone Redb log storage (for testing)
  - `storage_shared.rs`: SharedRedbStorage (unified log + state machine, single-fsync, production)
  - `network.rs`: IrpcRaftNetwork for Raft RPC over Iroh QUIC
  - `madsim_network.rs`: Deterministic simulation network for testing
  - `constants.rs`: Tiger Style resource limits and timeouts
  - `types.rs`: Type configurations for openraft
- **src/sql/**: DataFusion SQL integration over Redb KV data
  - `provider.rs`: RedbTableProvider implementing DataFusion TableProvider
  - `executor.rs`: RedbSqlExecutor for query execution
  - `schema.rs`: KV table schema definitions
- **src/cluster/** (2,889 lines): Cluster coordination and transport
  - `IrohEndpointManager`: Manages Iroh P2P endpoint lifecycle and discovery services
  - `bootstrap_simple.rs`: Node bootstrap orchestration with direct async APIs
  - `gossip_discovery.rs`: Iroh-gossip based peer discovery and announcements
  - `config.rs`: Multi-layer cluster configuration (env → TOML → CLI)
  - `metadata.rs`: Persistent node registry backed by redb
  - `ticket.rs`: Cluster tickets for convenient bootstrap sharing
- **src/node/** (234 lines): Node builder pattern
  - `NodeBuilder`: Fluent API for programmatic node configuration and startup
  - Returns handle with access to both trait implementations
- **src/bin/**:
  - `aspen-node.rs`: Full cluster node with Iroh-based client RPC (legacy HTTP API to be removed)
  - `aspen-tui/`: Terminal UI for cluster monitoring and management (requires `tui` feature)
  - `aspen-cli/`: Command-line client for interacting with Aspen clusters
  - `aspen-fuse/`: FUSE filesystem implementation (requires `fuse` feature)
  - `aspen-prometheus-adapter.rs`: Prometheus metrics adapter
  - `generate_fuzz_corpus.rs`: Fuzzing corpus generation utility (requires `fuzzing` feature)

## Development Commands

### Long-Running Commands

Use `pueue` for long-running commands (builds, tests, etc.) to prevent blocking the terminal:

```bash
# Queue a build
pueue add -- nix develop -c cargo build

# Queue tests
pueue add -- nix develop -c cargo nextest run

# Queue clippy
pueue add -- nix develop -c cargo clippy --all-targets -- --deny warnings

# Check progress
pueue status

# View output of a task
pueue log <id>
```

Pueue runs commands in the background and persists across terminal sessions.

### Building and Testing

```bash
# Build the project (inside Nix environment)
pueue add -- nix develop -c cargo build

# Run tests with cargo-nextest
pueue add -- nix develop -c cargo nextest run

# Run a specific test
nix develop -c cargo nextest run <test_name>

# Format code with Nix formatter
nix fmt

# Run clippy lints
pueue add -- nix develop -c cargo clippy --all-targets -- --deny warnings
```

### Quick Testing

For faster iteration during development, use the `quick` profile which skips slow tests (proptest, chaos, multi-seed madsim):

```bash
# Quick tests (~2-5 min instead of ~20-30 min)
nix develop -c cargo nextest run -P quick

# Run tests for specific module
nix develop -c cargo nextest run -E 'test(/raft/)'
nix develop -c cargo nextest run -E 'test(/storage/)'
nix develop -c cargo nextest run -E 'test(/gossip/)'

# Combine quick profile with module filter
nix develop -c cargo nextest run -P quick -E 'test(/raft/)'
```

### Test Results

After running tests, results are automatically saved to JUnit XML:

```
target/nextest/default/junit.xml
```

**Always read this file after test runs** to see detailed results including:

- Pass/fail status for each test
- Failure messages and stack traces
- Test duration
- Stdout/stderr for failed tests

### Running Scripts

```bash
# Run Aspen cluster smoke tests
./scripts/aspen-cluster-smoke.sh          # 3-node cluster with basic operations
./scripts/aspen-cluster-raft-smoke.sh     # Comprehensive Raft operations test (used in CI)

# DNS demo
./scripts/dns-demo.sh

# Multi-cluster demos
./scripts/cluster.sh                       # General cluster launcher
./scripts/kitty-cluster.sh                 # Launch cluster in kitty terminal tabs
```

### Running Benchmarks

```bash
# Run all benchmarks (preferred)
nix run .#bench

# Run benchmarks matching a filter
nix run .#bench kv_write
nix run .#bench kv_read

# Alternative: Run directly with cargo
nix develop -c cargo bench --bench kv_operations
nix develop -c cargo bench -- "kv_write"
```

**Benchmark results** are saved to `target/criterion/` with HTML reports.

**Current benchmark suites:**

- `kv_operations`: Synthetic benchmarks with in-memory storage (for protocol testing)
- `production`: Production-realistic benchmarks with Redb + Iroh networking

**Baseline metrics (production storage, Redb single-fsync + Iroh):**

- Single-node write: ~2-3 ms (350+ ops/sec) - single-fsync Redb
- Single-node read: ~10 us (100K ops/sec) - Redb + ReadIndex
- 3-node write: ~8-10 ms (100+ ops/sec) - single-fsync + Iroh QUIC quorum
- 3-node read: ~36 us (27K ops/sec) - ReadIndex over network

**Synthetic benchmarks (in-memory, for protocol testing only):**

- In-memory write: ~34 us (no disk, no network - NOT production representative)
- In-memory read: ~170 ns (BTreeMap lookup - NOT production representative)

### Nix Development

```bash
# Enter the Nix development shell
nix develop

# Run commands in Nix environment without entering shell
nix develop -c <command>

# Check flake (runs tests + lints)
nix flake check

# Build specific outputs
nix build .#<output>

# Code coverage
nix run .#coverage              # Show summary
nix run .#coverage html         # Generate HTML report and open browser
nix run .#coverage ci           # Generate lcov.info for CI
nix run .#coverage update       # Update .coverage-baseline.toml

# Formatting
nix fmt                         # Format all Nix files with alejandra
nix run .#rustfmt              # Format Rust files with nightly rustfmt
nix run .#rustfmt check        # Check Rust formatting

# Fuzzing
nix run .#fuzz                 # Parallel fuzzing (1hr/target)
nix run .#fuzz-quick           # Quick smoke test (5min/target)
nix run .#fuzz-overnight       # Overnight run (8hr, 4 parallel targets)
nix run .#fuzz-intensive       # Full campaign (6hr/target, sequential)
nix run .#fuzz-coverage        # Generate coverage report for fuzz targets
nix run .#fuzz-corpus          # Generate seed corpus

# Launch cluster demos
nix run .#cluster              # 3-node cluster
nix run .#kitty-cluster        # Cluster in kitty terminal tabs
```

## Feature Flags

Aspen uses Cargo feature flags to enable optional functionality:

- **default**: `["sql", "dns"]` - SQL queries and DNS protocol support enabled by default
- **sql**: DataFusion SQL query engine for Redb storage backend
- **dns**: DNS record management layer with hickory-server protocol support
- **tui**: Terminal UI with ratatui (required for `aspen-tui` binary)
- **fuse**: FUSE filesystem support (required for `aspen-fuse` binary)
- **virtiofs**: VirtioFS support for VM guests (requires `fuse` feature)
- **global-discovery**: Global content discovery via BitTorrent Mainline DHT
- **fuzzing**: Exposes internals for fuzz testing (dev/test only)
- **bolero**: Bolero property-based testing (dev/test only)
- **testing**: Test-specific utilities

**Building with specific features:**

```bash
# Build with all default features
cargo build

# Build without default features
cargo build --no-default-features

# Build with specific features
cargo build --features "tui,fuse"

# Build TUI binary (requires tui feature)
cargo build --bin aspen-tui --features tui

# Build FUSE binary (requires fuse feature)
cargo build --bin aspen-fuse --features fuse
```

## Nextest Configuration

Aspen uses cargo-nextest with custom profiles defined in `.config/nextest.toml`:

**Profiles:**

- **default**: Standard test profile with 1 retry, 60s timeout for slow tests, fail-fast enabled
- **quick**: Fast iteration profile - skips slow tests (proptest, chaos, multi-seed madsim, crash recovery)
  - Usage: `cargo nextest run -P quick`
  - Filter: Excludes `/proptest/`, `/chaos/`, `/madsim_multi/`, `/multiple_crash/`, `/buggify/`, `/real_crash/`
- **ci**: CI profile with extended timeouts (120s) for property-based tests, fail-fast disabled

**Test overrides:**

- Property-based tests (`test_proptest_*`): 120s timeout, 1 retry
- Multi-crash recovery tests: 120s timeout, 2 retries
- BUGGIFY stress tests: 120s timeout, 2 retries
- Subprocess crash tests (`/real_crash/`): 180s timeout, 1 retry

**JUnit output:**

All test runs automatically generate JUnit XML at `junit.xml` (or `target/nextest/default/junit.xml`) with failure output captured.

## Coding Style: Tiger Style

Aspen follows "Tiger Style" principles (see tigerstyle.md):

### Safety Principles

- Avoid `.unwrap()` and `.expect()` in production code; use `?` operator, `context()`/`whatever_context()` from snafu, or explicit pattern matching instead
- Avoid `panic!()`, `todo!()`, `unimplemented!()` in production code
- Avoid `unsafe` unless absolutely necessary; document safety invariants when used
- Use explicitly sized types (`u32`, `i64`) instead of `usize` for portability
- Set fixed limits on loops, queues, and data structures to prevent unbounded resource use
- Static memory allocation preferred; avoid dynamic allocation after initialization
- Minimize variable scope; declare variables close to their usage
- Use assertions to verify invariants and function pre/post-conditions
- Fail fast on programmer errors; handle all errors explicitly
- Centralize control flow in parent functions; move non-branching logic to helper functions
- Keep functions under 70 lines

### Async/Concurrency Safety

- Avoid holding locks across `.await` points (causes deadlocks)
- Prefer `tokio::sync` primitives over `std::sync` in async code
- Use structured concurrency (`JoinSet`, `TaskTracker`) for spawned tasks
- Be explicit about cancellation safety in async code
- Use `tokio::select!` carefully; understand which branches are cancel-safe

### Memory/Allocation

- Avoid unnecessary `.clone()`; prefer borrowing when possible
- Use `&str` over `String` in function parameters where ownership isn't needed

### Testability

- Prefer pure functions (deterministic output from inputs, no side effects) for easier testing
- Extract complex logic into pure functions that can be unit tested independently

### Error Handling

- Add context to errors (use snafu's `context()`) so failures are actionable
- Don't log and return the same error (causes duplicate noise)

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

### Naming and Parameter Conventions

Follow Rust API Guidelines (https://rust-lang.github.io/api-guidelines/naming.html):

**Casing:**

- Types, traits, enum variants: `UpperCamelCase` (acronyms as words: `Uuid`, not `UUID`)
- Functions, methods, variables, modules: `snake_case`
- Constants and statics: `SCREAMING_SNAKE_CASE`

**Conversion methods (C-CONV):**

- `as_*`: Free conversion, borrowed to borrowed (`as_bytes`, `as_mut_slice`)
- `to_*`: Expensive conversion, borrowed to owned (`to_string`, `to_vec`)
- `into_*`: Owned to owned, consumes self (`into_inner`, `into_bytes`)

**Getters (C-GETTER):**

- Omit `get_` prefix: use `fn len()` not `fn get_len()`
- Use `*_mut()` suffix for mutable access: `fn first_mut()`

**Iterator methods (C-ITER):**

- `iter(&self)` returns `Iter` yielding `&T`
- `iter_mut(&mut self)` returns `IterMut` yielding `&mut T`
- `into_iter(self)` returns `IntoIter` yielding `T`

**Function parameter ordering:**

NOTE: rustfmt cannot enforce parameter ordering; this is enforced through code review.

Preferred order (most to least important):

1. `&self` / `&mut self` / `self` (receiver always first)
2. Borrowed references (`&T`, `&str`, `&[T]`) - cheap to pass, no ownership transfer
3. Small Copy types (`u32`, `bool`, `NodeId`) - pass by value
4. Owned types (`String`, `Vec<T>`, `Box<T>`) - ownership transfer
5. Closures and callbacks (`F: FnOnce(...)`) - always last

Examples:

```rust
// Good: borrowed refs before owned types, closure last
fn process(name: &str, id: u32, data: Vec<u8>, callback: impl FnOnce())

// Good: self first, then references, then owned
fn update(&mut self, prefix: &[u8], new_value: String)

// Avoid: owned before borrowed (harder to call, unclear ownership)
fn process(data: Vec<u8>, name: &str)  // prefer: (name: &str, data: Vec<u8>)
```

**Variable naming with units:**

- Include units in descending specificity: `timeout_ms`, `size_bytes`, `latency_us_max`
- Boolean prefixes: `is_`, `has_`, `should_`, `can_` (`is_leader`, `has_quorum`)

**Error type naming:**

- Pattern: `VerbObjectError` (`ParseConfigError`, `ConnectNodeError`)
- Not: `ObjectVerbError` or `ErrorVerb`

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
  - References:
    - [FoundationDB Testing Approach](https://apple.github.io/foundationdb/testing.html) - Industry-leading deterministic simulation testing
    - [Sled Simulation Testing](https://sled.rs/simulation.html) - Practical simulation testing for Rust databases
    - [Deterministic Simulation: A New Era of Distributed System Testing](https://www.risingwave.com/blog/deterministic-simulation-a-new-era-of-distributed-system-testing/) - RisingWave's comprehensive guide to simulation testing
    - [Applying Deterministic Simulation: The RisingWave Story](https://www.risingwave.com/blog/applying-deterministic-simulation-the-risingwave-story-part-2-of-2/) - Real-world implementation lessons from RisingWave
    - [TigerBeetle VOPR](https://github.com/tigerbeetle/tigerbeetle/blob/main/docs/internals/vopr.md) - Viewstamped Operation Protocol Replication design and testing methodology
    - [TigerBeetle Architecture](https://github.com/tigerbeetle/tigerbeetle/blob/main/docs/internals/ARCHITECTURE.md) - Financial database architecture with Tiger Style principles
    - [TigerBeetle VSR](https://github.com/tigerbeetle/tigerbeetle/blob/main/docs/internals/vsr.md) - Viewstamped Replication consensus protocol implementation
    - [TigerBeetle Data File](https://github.com/tigerbeetle/tigerbeetle/blob/main/docs/internals/data_file.md) - Storage engine design for deterministic performance
- **Property-based testing**: Use `proptest` for exploring edge cases
- **Unified testing**: Use `bolero` for tests that can run as both property-based tests (in CI) and fuzz tests (for deeper exploration)
- **Test modifications**: Never modify, remove, or add tests unless explicitly asked

## Core API Traits

### ClusterController

Manages cluster membership and consensus operations. Key responsibilities:

- **init()**: Initialize a new cluster with founding members
- **add_learner()**: Add non-voting nodes that replicate data but don't participate in consensus
- **change_membership()**: Reconfigure the set of voting members
- **current_state()**: Get current cluster topology (nodes, voters, learners)
- **get_metrics()**: Access raw OpenRaft metrics for monitoring
- **trigger_snapshot()**: Manually trigger state machine snapshot
- **get_leader()**: Convenience method to identify current Raft leader

### KeyValueStore

Provides distributed key-value operations with linearizable consistency:

- **write()**: Apply writes through Raft consensus (Set, SetMulti, Delete, DeleteMulti)
- **read()**: Linearizable reads via ReadIndex protocol (falls back to local if leader unavailable)
- **delete()**: Remove keys through consensus (idempotent operations)
- **scan()**: Prefix-based scanning with pagination support (max 10,000 results)

Both traits are implemented by `RaftNode` for production use and have deterministic in-memory implementations for testing.

## Current Implementation Status

- **Lines of Code**: ~21,000 production code + extensive tests
- **Test Coverage**: 350+ tests (unit, integration, simulation, property-based)
- **Compilation**: Clean build with 0 warnings
- **Stubs/Placeholders**: NONE - all code is fully implemented
- **Recent Refactoring**: Removed actor-based architecture (Dec 13, 2025) for simpler direct async APIs

## Tiger Style Resource Bounds

The codebase enforces explicit limits to prevent resource exhaustion:

- `MAX_BATCH_SIZE` = 1,000 entries
- `MAX_SCAN_RESULTS` = 10,000 keys
- `MAX_KEY_SIZE` = 1 KB
- `MAX_VALUE_SIZE` = 1 MB
- `MAX_CONCURRENT_OPS` = 1,000
- `MAX_PEERS` = 64
- `MAX_CONNECTIONS` = 500
- Connection timeouts: 5s connect, 2s stream, 10s read

## Git Commit Guidelines

- **Never add co-author attribution** to commits. Do not include `Co-Authored-By` trailers or any AI attribution in commit messages.
- Commit messages should be concise and focus on the "why" rather than the "what"
- Follow conventional commit style when appropriate

## Important Notes

- The codebase underwent a major architectural simplification from actor-based (ractor) to direct async APIs (Dec 13, 2025)
- All references to `RaftActor`, `RaftControlClient`, `KvServiceBuilder`, or `NodeServerHandle` in documentation are outdated
- Keep Iroh P2P coupling as-is; it's a core infrastructure service
- Backwards compatibility is not a concern; prioritize clean, modern solutions
- The project contains a vendored/embedded `openraft` directory (v0.10.0) with no local modifications
- Edition is set to `2024` in Cargo.toml (Rust 2024 edition)
- The system is production-ready and fully functional
- Recent additions (Dec 2025):
  - Optional feature gates for `sql` and `dns` (both enabled by default)
  - Global content discovery via BitTorrent Mainline DHT with BEP-44 mutable items
  - FUSE filesystem support for mounting Aspen as a filesystem
  - Enhanced nextest configuration with `quick` profile for fast iteration
  - Comprehensive Nix flake apps for coverage, fuzzing, and formatting
