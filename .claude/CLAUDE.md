# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Aspen is a foundational orchestration layer for distributed systems, written in Rust. It provides distributed primitives for managing and coordinating distributed systems, drawing inspiration from Erlang/BEAM, Plan9, Kubernetes, FoundationDB, etcd, and Antithesis.

**Status**: Production-ready with ~311,000 lines of code across 33 crates and 350+ passing tests. All trait-based APIs have complete implementations with no stubs or placeholders. The codebase uses direct async APIs (actor-based architecture was removed Dec 13, 2025).

**Goal: Self-Hosted Infrastructure** - Aspen aims to build and host itself using its own distributed primitives:

- **Forge**: Decentralized Git hosting for Aspen's source code (replaces GitHub/GitLab)
- **Aspen CI**: Distributed CI/CD pipeline for building and testing Aspen (replaces GitHub Actions)
- **Nix**: Reproducible builds with artifacts stored in Aspen's distributed binary cache

This "eating our own dog food" approach ensures Aspen is robust enough for production use and demonstrates the full capabilities of the platform.

## Core Technologies

- **openraft v0.10.0**: Raft consensus (vendored at `openraft/openraft`)
- **redb**: Unified Raft log + state machine storage (single-fsync writes, ~2-3ms latency)
- **iroh**: P2P networking via QUIC (all client and inter-node communication)
- **iroh-blobs/iroh-docs**: Content-addressed blob storage and CRDT replication
- **madsim**: Deterministic simulation testing
- **DataFusion**: SQL over KV data (optional, `sql` feature)
- **snafu/anyhow**: Error handling (snafu for library, anyhow for application)

### Feature Flags

Default features (enabled in production):

- **sql**: DataFusion SQL engine
- **dns**: DNS with hickory-server
- **forge/git-bridge**: Git hosting and bidirectional sync
- **pijul**: Pijul VCS integration
- **ci**: CI/CD pipelines
- **secrets**: SOPS secrets management
- **automerge**: CRDT documents
- **global-discovery**: BitTorrent DHT
- **vm-executor/shell-worker**: Job isolation

Dev features: testing, fuzzing, bolero, snix, nix-cache-gateway

## Vendored openraft

Vendored at `openraft/openraft` v0.10.0 for tight control over consensus layer.

**Update procedure**:

1. Review upstream releases for relevant changes
2. Cherry-pick/rebase local modifications onto new version
3. Run full test suite (especially madsim tests)
4. Document modifications in commit messages

## Architecture

**Core components** (all required):

1. **Raft (openraft)**: Cluster-wide consensus, linearizable operations
2. **Iroh P2P**: All inter-node networking, discovery, NAT traversal
3. **Storage (redb)**: Unified log + state machine, single-fsync writes

**Key traits**:

- `ClusterController`: Cluster membership (init, add learner, change membership, metrics)
- `KeyValueStore`: Distributed KV operations (read, write, delete, scan)

Both implemented by `RaftNode` for production; deterministic in-memory versions for testing.

### API Design: Iroh-Only

**No HTTP API** - all communication uses Iroh QUIC with ALPN-based protocol routing:

- Client APIs: `CLIENT_ALPN` via `ClientProtocolHandler`
- Node-to-node: ALPN routing (RAFT_ALPN, TUI_ALPN, GOSSIP_ALPN)
- Blob transfer: iroh-blobs protocol
- Real-time sync: iroh-docs CRDT replication

### Implementation Rules

**For new distributed features:**

1. Use Iroh for ALL network communication (no raw TCP/HTTP)
2. Go through Raft consensus for cluster-wide state
3. Use trait-based API (`ClusterController`, `KeyValueStore`)
4. Follow Tiger Style resource bounds (see `crates/aspen-core/src/constants/`)

**Never:**

- Add HTTP endpoints or use axum (use Iroh Client RPC)
- Implement node-to-node communication without Iroh
- Create distributed state without Raft consensus
- Allow unbounded operations (always use Tiger Style limits)
- Mix transport layer (Iroh) with application layer (Raft membership)

### Key Modules

- **crates/aspen-core/**: Core types, traits (`ClusterController`, `KeyValueStore`), and constants
- **crates/aspen-raft/**: Raft consensus (~22,000 lines) - `RaftNode`, storage, network
- **crates/aspen-coordination/**: Distributed primitives (queues, locks, barriers)
- **crates/aspen-rpc-handlers/**: Central RPC infrastructure for protocol handlers
- **crates/aspen-cluster/**: Cluster coordination - `IrohEndpointManager`, bootstrap, gossip
- **crates/aspen-transport/**: Transport layer with ALPN constants
- **crates/aspen-jobs/**: Job execution system with VM/shell workers
- **crates/aspen-ci/**: CI/CD system with Nickel config
- **crates/aspen-forge/**: Forge - decentralized Git hosting
- **crates/aspen-client-api/**: Client RPC protocol definitions
- **crates/aspen-sql/**: DataFusion SQL over Redb KV (optional)
- **crates/aspen-tui/**: Terminal UI (separate binary)
- **crates/aspen-cli/**: Command-line client (separate binary)

See `docs/architecture/` for detailed design documents.

## Development Commands

```bash
# Enter shell once (Mold linker, incremental compilation, all dev tools)
nix develop

# Build and test
cargo build                              # Fast incremental (~2-3s rebuilds)
cargo nextest run                        # Run all tests
cargo nextest run -P quick               # Quick tests (~2-5 min, skips proptest/chaos)
cargo nextest run -E 'test(/raft/)'      # Tests for specific module
cargo nextest run <test_name>            # Single test
cargo nextest run -P network --run-ignored all  # Tests requiring real network

# Linting and formatting
cargo clippy --all-targets -- --deny warnings
nix run .#rustfmt                        # Format Rust (IMPORTANT: use this, not cargo fmt)

# Run binaries
cargo run --bin aspen-node -- --node-id 1 --cookie my-cluster
cargo run --bin aspen-cli -- kv get mykey
```

**Test results**: JUnit XML at `target/nextest/default/junit.xml`

### Nextest Profiles

- **default**: Standard with 60s timeout, fail-fast
- **quick**: Skips slow tests (proptest, chaos, madsim_multi, crash recovery)
- **ci**: Extended 120s timeouts, fail-fast disabled
- **network**: For tests requiring real network access

### Nix Apps (without entering shell)

```bash
nix flake check                # Full verification
nix run .#coverage html        # Code coverage report
nix run .#fuzz-quick           # Quick fuzz testing (5min/target)
nix run .#cluster              # Launch 3-node cluster
```

### Binaries

- **aspen-node**: Cluster node server
- **aspen-cli**: Command-line client
- **aspen-tui**: Terminal UI
- **aspen-ci-agent**: CI job agent
- **aspen-fuse**: FUSE filesystem
- **git-remote-aspen**: Git remote helper
- **aspen-generate-schema**: Schema generator

## Coding Style: Tiger Style

See `tigerstyle.md` for full principles. Key rules for this codebase:

**Safety:**

- No `.unwrap()`/`.expect()` in production; use `?` with snafu `context()`
- No `panic!()`/`todo!()`/`unimplemented!()` in production
- Use explicitly sized types (`u32`, `i64`) not `usize`
- Keep functions under 70 lines
- Set fixed limits on loops/queues/data structures

**Async safety:**

- Never hold locks across `.await` points
- Use `tokio::sync` over `std::sync` in async code
- Use `JoinSet`/`TaskTracker` for spawned tasks

**Error handling:**

- Add context via snafu's `context()` for actionable errors
- Don't log and return the same error

**Naming:**

- Include units: `timeout_ms`, `size_bytes`, `latency_us_max`
- Boolean prefixes: `is_`, `has_`, `should_`, `can_`
- Error types: `VerbObjectError` (e.g., `ParseConfigError`)
- Parameter order: `&self` → borrowed refs → Copy types → owned → closures

**Functional Core, Imperative Shell:**

Aspen implements FCIS via `verified` modules in core crates:

- **Verified functions** (`src/verified/`): Deterministic, no I/O, no async, time as explicit parameter
- **Shell layer**: Async handlers that call verified functions, manage I/O, and interact with stores

Benefits:

- Verus formal verification (see `verus/` directories for specs)
- madsim deterministic simulation (no hidden time dependencies)
- Property-based testing with proptest/Bolero
- WASM compilation for portable logic

Architecture:

- **src/verified/**: Production exec functions compiled normally by cargo
- **verus/**: Standalone Verus specs with ensures/requires clauses verified by Verus
- Backwards compatibility: `pub use verified as pure` allows legacy imports

Pattern from `aspen-coordination`:

```rust
// Verified core (src/verified/lock.rs)
fn compute_next_fencing_token(current: Option<&LockEntry>) -> u64
fn is_lock_expired(deadline_ms: u64, now_ms: u64) -> bool

// Imperative shell (src/lock.rs)
async fn try_acquire(&self) -> Result<LockGuard<S>, CoordinationError>
```

When adding features:

1. Extract business logic into `src/verified/` as deterministic functions
2. Pass time, randomness, and configuration as explicit parameters
3. Keep shell thin: only I/O, async coordination, and error context
4. Unit tests for verified functions, integration tests for shell
5. Add Verus specs in `verus/` directory for formal verification

## Testing Philosophy

- **Integration over unit tests**: Real services, not mocks
- **Deterministic simulation**: `madsim` for reproducible distributed tests
- **Property-based testing**: `proptest` for edge cases, `bolero` for unified fuzz/property tests
- **Never modify tests** unless explicitly asked

## Resource Bounds

Fixed limits in `crates/aspen-core/src/constants/` to prevent resource exhaustion:

- `MAX_BATCH_SIZE` = 1,000 entries
- `MAX_SCAN_RESULTS` = 10,000 keys
- `MAX_KEY_SIZE` = 1 KB, `MAX_VALUE_SIZE` = 1 MB
- `MAX_PEERS` = 1,000, `MAX_CONCURRENT_CONNECTIONS` = 500
- Timeouts: 5s connect, 2s stream, 10s read

## Self-Hosting Architecture

Aspen's self-hosting strategy uses three integrated components:

### Forge (Decentralized Git)

Git hosting via `git-remote-aspen` helper:

```bash
# Clone Aspen from an Aspen cluster
git clone aspen://<ticket>/<repo_id> aspen
cd aspen && git push aspen main
```

**Components**: GitBlobStore (iroh-blobs), RefStore (Raft KV), COB system (issues, patches), gossip announcements

### Aspen CI (Distributed CI/CD)

Pipeline configuration in `.aspen/ci.ncl` (Nickel):

```nickel
{
  stages = [
    { name = "check", jobs = [{ type = 'shell, command = "cargo fmt --check" }] },
    { name = "build", jobs = [{ type = 'nix, flake_attr = "packages.x86_64-linux.default" }] },
    { name = "test", jobs = [{ type = 'shell, command = "cargo nextest run" }] },
  ]
}
```

**Components**: TriggerService (gossip-triggered builds), PipelineOrchestrator, NixBuildWorker, artifact storage in iroh-blobs

### Nix Integration

Built store paths automatically registered in Aspen's distributed binary cache:

- NAR archives stored in iroh-blobs (P2P distribution)
- CacheIndex tracks store paths with metadata
- SNIX storage for decomposed content-addressed storage
- Workers share build artifacts across cluster

### Self-Hosting Roadmap

1. **Phase 1** (Complete): Core Forge and CI implementations
2. **Phase 2** (In Progress): Integration testing, real cluster deployments
3. **Phase 3** (Planned): Full dogfooding - host Aspen development on Aspen
4. **Phase 4** (Planned): Public Aspen instance for external contributors

## Git Commit Guidelines

- **Never add co-author attribution** (no `Co-Authored-By` trailers)
- Concise messages focusing on "why" not "what"
- Conventional commit style when appropriate

## Important Notes

- Rust 2024 edition
- Backwards compatibility is not a concern; prioritize clean solutions
- Outdated references: `RaftActor`, `RaftControlClient`, `KvServiceBuilder`, `NodeServerHandle`
