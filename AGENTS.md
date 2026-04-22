# AGENTS.md

This file provides guidance when working with code in this repository.

## Project Overview

Aspen is a foundational orchestration layer for distributed systems, written in Rust. It provides distributed primitives for managing and coordinating distributed systems, drawing inspiration from Erlang/BEAM, Plan9, Kubernetes, FoundationDB, etcd, and Antithesis.

**Status**: Production-ready with ~436,000 lines of Rust across 80 crates. 44 NixOS VM integration tests, 5,700+ unit/integration tests. All trait-based APIs have complete implementations with no stubs or placeholders.

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
- **snix**: Nix integration via snix crates (eval, build, store, daemon protocol)
- **madsim**: Deterministic simulation testing
- **DataFusion**: SQL over KV data (optional, `sql` feature)
- **snafu/anyhow**: Error handling (snafu for library, anyhow for application)

### Feature Flags

Default features are empty — users opt-in as needed. The `full` feature enables everything:

- **sql**: DataFusion SQL engine
- **forge/git-bridge**: Git hosting and bidirectional sync
- **ci/ci-basic**: CI/CD pipelines (basic = shell executor, full = VM executor, deploy stages via `DeployExecutor`)
- **secrets**: SOPS secrets management
- **automerge**: CRDT documents
- **global-discovery**: BitTorrent DHT
- **plugins**: Hyperlight-based execution (VM, WASM, RPC plugins)
- **shell-worker**: Shell command job execution
- **blob/docs**: iroh-blobs storage and iroh-docs CRDT replication
- **hooks**: Event hook system
- **jobs**: Job execution framework
- **federation**: Cross-cluster federation
- **proxy**: Reverse proxy
- **snix**: Content-addressed store via snix-castore/snix-store (BlobService, DirectoryService, PathInfoService)
- **snix-http**: nar-bridge HTTP server for Nix binary cache protocol
- **snix-daemon**: nix-daemon Unix socket protocol (nix path-info, nix copy)
- **snix-eval**: In-process Nix evaluation via snix-eval/snix-glue (flake eval, config parsing via snix-serde)
- **snix-build**: Native build execution via snix-build BuildService (bubblewrap/OCI sandbox, replaces `nix build` subprocess). Includes `reqwest` for in-process HTTP, `xz2`/`zstd`/`bzip2` for NAR decompression, and `UpstreamCacheClient` for fetching from cache.nixos.org
- **trust**: Shamir cluster secret sharing — GF(2^8) split/reconstruct, HKDF key derivation, redb share storage (see `docs/trust-quorum.md`)
- **nix-cli-fallback**: Opt-in last-resort subprocess calls (`nix eval`, `nix build`, `nix path-info`, `nix-store -qR`, `nix-store --realise`, `nix flake lock`, `curl`). Without this feature, the native snix pipeline is the only build path.

Dev features: testing, fuzzing, bolero, simulation

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
- **crates/aspen-raft/**: Raft consensus (~30,000 lines) - `RaftNode`, storage, network
- **crates/aspen-coordination/**: Distributed primitives (queues, locks, barriers)
- **crates/aspen-rpc-handlers/**: Central RPC infrastructure for protocol handlers
- **crates/aspen-cluster/**: Cluster coordination - `IrohEndpointManager`, bootstrap, gossip
- **crates/aspen-transport/**: Transport layer with ALPN constants
- **crates/aspen-jobs/**: Job execution system with VM/shell workers
- **crates/aspen-ci/**: CI/CD system with Nickel config, deploy executor (`orchestrator/deploy_executor.rs`)
- **crates/aspen-forge/**: Forge - decentralized Git hosting
- **crates/aspen-client-api/**: Client RPC protocol definitions
- **crates/aspen-sql/**: DataFusion SQL over Redb KV (optional)
- **crates/aspen-snix/**: Raft-backed snix trait impls (BlobService, DirectoryService, PathInfoService)
- **crates/aspen-snix-bridge/**: gRPC + nix-daemon bridge between snix-store and Aspen cluster
- **crates/aspen-nix-cache-gateway/**: HTTP binary cache gateway (nar-bridge backed)
- **crates/aspen-ci-executor-nix/**: Nix build executor with snix integration (eval, build, cache upload)
- **crates/aspen-tui/**: Terminal UI (separate binary)
- **crates/aspen-cli/**: Command-line client (separate binary)

See `docs/` for design documents (federation, forge, plugins, simulations, nix integration).

## Observability

Metrics use the `metrics` crate facade with a Prometheus recorder installed at startup. All metric names use dot-separated namespaces (the recorder converts dots to underscores in Prometheus output).

**Key metrics:**

| Metric | Type | Labels | Source |
|--------|------|--------|--------|
| `aspen.rpc.requests_total` | counter | `operation` | `HandlerRegistry::dispatch` |
| `aspen.rpc.duration_ms` | histogram | `operation`, `handler` | `HandlerRegistry::dispatch` |
| `aspen.rpc.errors_total` | counter | `operation`, `handler` | `HandlerRegistry::dispatch` |
| `aspen.write_batcher.batch_size` | histogram | | `WriteBatcher::flush_batch` |
| `aspen.write_batcher.flush_total` | counter | | `WriteBatcher::flush_batch` |
| `aspen.write_batcher.flush_duration_ms` | histogram | | `WriteBatcher::flush_batch` |
| `aspen.write_batcher.forwarded_total` | counter | | follower forwarding path |
| `aspen.snapshot.transfer_size_bytes` | histogram | `direction`, `peer` | snapshot send/receive |
| `aspen.snapshot.transfer_duration_ms` | histogram | `direction` | snapshot send/receive |
| `aspen.snapshot.transfers_total` | counter | `direction`, `outcome` | snapshot send/receive |
| `aspen.network.connections` | gauge | `state` (healthy/degraded/failed) | periodic 10s task |
| `aspen.network.active_streams` | gauge | | periodic 10s task |
| `aspen.raft.term` | gauge | `node_id` | `GetMetrics` handler |
| `aspen.raft.state` | gauge | `node_id` | `GetMetrics` handler |
| `aspen.node.uptime_seconds` | gauge | `node_id` | `GetMetrics` handler |

**Querying metrics:**

- `GetMetrics` RPC → Prometheus text format (all registered metrics)
- `GetNetworkMetrics` RPC → connection pool stats + snapshot transfer history
- `aspen-cli cluster prometheus` → raw Prometheus output
- `aspen-cli cluster network` → formatted pool/stream/snapshot summary

**OTLP export** (optional, `otlp` feature flag):

```bash
cargo run --features otlp --bin aspen-node -- --otlp-endpoint http://localhost:4317
```

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
# Note: `cargo nextest run -P quick --workspace` currently pulls vendored iroh crates
# (`vendor/iroh-h3-axum`, `vendor/iroh-proxy-utils`) whose tests/examples don't build
# against the pinned iroh 0.97 API. Exclude those crates when auditing Aspen proper.

# Mutation testing (verify tests catch real bugs)
cargo mutants -p aspen-coordination --timeout 60  # Baseline: coordination
cargo mutants -p aspen-core --timeout 60           # Baseline: core

# Serialization snapshot tests
cargo insta test                                   # Run snapshot tests
cargo insta review                                 # Review changed snapshots
INSTA_UPDATE=always cargo test                     # Auto-accept new snapshots

# Linting and formatting
cargo clippy --all-targets -- --deny warnings
nix run .#rustfmt                        # Format Rust (IMPORTANT: use this, not cargo fmt)
# Tiger Style: all 32 lints enabled workspace-wide via cargo-tigerstyle.
# In devShell: `cargo tigerstyle check` runs all lints.
# CI: `nix flake check` includes `checks.tigerstyle-check`.
# Script: `scripts/tigerstyle-check.sh` wraps cargo-tigerstyle with fallback.
# Lint levels and thresholds are configured in `dylint.toml`.
# Allow-by-default lints are promoted to warn for full coverage.
# `cargo-tigerstyle` first-class config is in `[workspace.metadata.tigerstyle]`.
# `aspen-time` owns the wall-clock boundary, so direct clock reads there use
# item-scoped `#[allow(ambient_clock, reason = "...")]`. Elsewhere, route
# wall-clock reads through `aspen_time::current_time_ms()` or a local helper.
# Note: the rustfmt hook runs repo-wide `nix run .#rustfmt`, not file-scoped formatting.
# Stash or restore unrelated unstaged Rust edits before making a focused commit.

# Run binaries
cargo run --features jobs,docs,blob,hooks,automerge --bin aspen-node -- --node-id 1 --cookie my-cluster
cargo run -p aspen-cli --bin aspen-cli -- kv get mykey
```

**Test results**: JUnit XML at `target/nextest/default/junit.xml`

### Nextest Profiles

- **default**: Standard with 60s timeout, fail-fast
- **quick**: Skips slow tests (proptest, chaos, madsim_multi, crash recovery)
- **ci**: Extended 120s timeouts, fail-fast disabled
- **network**: For tests requiring real network access
- **patchbay**: Namespace-backed patchbay suites (use `scripts/test-harness.sh list --layer patchbay --format commands` to resolve the generated commands)
- **snix**: All snix/nix-build tests (use `scripts/test-snix.sh`)

### Harness Metadata

- Suite manifests live under `test-harness/suites/` with the Nickel schema at `test-harness/schema.ncl`.
- `scripts/test-harness.sh export` regenerates `test-harness/generated/inventory.json`; `scripts/test-harness.sh check` fails on stale or invalid metadata.
- `scripts/test-harness.sh list ...` and `scripts/test-harness.sh run ...` now call the freshness check before reading the generated inventory.
- `flake.nix` rejects stale `test-harness/generated/inventory.json` during eval by comparing the committed metadata hash and manifest path list against the current Nickel inputs.
- When proving freshness enforcement, capture end-to-end failures at the consumer boundary: temporarily corrupt `test-harness/generated/inventory.json`, show `scripts/test-harness.sh list ...` fails, then restore the file before continuing.

### Nix Apps (without entering shell)

```bash
nix flake check                # Full verification
nix run .#coverage html        # Code coverage report
nix run .#fuzz-quick           # Quick fuzz testing (5min/target)
nix run .#cluster              # Launch 3-node cluster
nix run .#dogfood-local        # Full self-hosted build pipeline
nix run .#dogfood-local -- start  # Just start the cluster
```

### Dogfood (Self-Hosted Build)

Aspen builds itself using its own Forge + CI + Nix pipeline. The `aspen-dogfood` binary (`crates/aspen-dogfood/`) orchestrates the full pipeline using typed `aspen-client` RPCs over Iroh.

```bash
# Full pipeline: cluster → forge repo → git push → CI auto-trigger → nix build → deploy → verify → stop
nix run .#dogfood-local -- full      # complete pipeline: start → push → build → deploy → verify → stop

# Step-by-step
nix run .#dogfood-local -- start      # Start 1-node cluster
nix run .#dogfood-local -- push       # Push source to Forge
nix run .#dogfood-local -- build      # Wait for CI pipeline
nix run .#dogfood-local -- deploy     # Deploy CI-built artifact to cluster
nix run .#dogfood-local -- verify     # Verify deployed artifact
nix run .#dogfood-local -- full-loop  # build → deploy → verify (cluster already running)
nix run .#dogfood-local -- stop       # Clean up

# Modes
nix run .#dogfood-federation          # Two-cluster federation mode
nix run .#dogfood-local-vmci          # VM-isolated CI execution
```

Binary: `crates/aspen-dogfood/`. Old shell scripts in `scripts/deprecated/`.


### NixOS VM Integration Tests

44 NixOS VM tests in `nix/tests/`. Run with:

```bash
nix build .#checks.x86_64-linux.ci-dogfood-test --impure --option sandbox false
nix build .#checks.x86_64-linux.kv-operations-test
```

Key dogfood tests:

- `dogfood-binary-smoke-test`: Directly exercises `aspen-dogfood start/status/push/stop` in a VM. Uses `bins.ci-aspen-node-snix-build` so Forge + `CiWatchRepo` are available, and lives in the unconditional `system == "x86_64-linux"` checks block so it evaluates without `--impure` sibling repos.
- Manual federation smoke: `nix run .#dogfood-federation -- --cluster-dir /tmp/aspen-dogfood-federation-proof start`, then `status`, inspect `dogfood-state.json`, then `stop`. Useful for proving the alice/bob federation path without running the full build pipeline. In this environment `nix run` may print `unit2nix` evaluation warnings and iroh may log relay/DNS/IPv6 warnings before the final status lines; record those warnings instead of trimming them away.
- `ci-dogfood-test`: Push flake to Forge → CI auto-trigger → nix build → run result
- `ci-dogfood-self-build`: Push Aspen workspace to Forge → CI builds Aspen itself
- `ci-nix-build-test`: Single nix build job end-to-end

**Ad-hoc VM testing via pi tools** (no NixOS test framework rebuild needed):

- `vm_boot` + `vm_serial` for QEMU-based interactive testing
- `vm_serial command:` to run commands, `vm_serial expect:` to wait for patterns
- Boot milestones: `"Welcome to NixOS"` → `"login:"` → `"[#$] "`
- `vm_screenshot` for graphical debugging

**Serial dogfood VM** (standalone bootable NixOS with aspen):

```bash
nix build .#dogfood-serial-vm
cp result/disk.qcow2 /tmp/dogfood-serial.qcow2 && chmod +w /tmp/dogfood-serial.qcow2
# Then in pi:
#   vm_boot image="/tmp/dogfood-serial.qcow2" format="qcow2" memory="4096M" cpus=2
#   vm_serial expect:"root@dogfood"
#   vm_serial command:"/etc/dogfood/start-node.sh"
#   vm_serial command:"/etc/dogfood/cowsay-test.sh 2>&1"
```

### Binaries

- **aspen-node**: Cluster node server (requires `jobs,docs,blob,hooks,automerge` features)
- **aspen-cli**: Command-line client (separate crate: `aspen-cli`)
- **aspen-tui**: Terminal UI (separate crate: `aspen-tui`)
- **aspen-ci-agent**: CI job agent (separate crate: `aspen-ci`)
- **aspen-fuse**: FUSE filesystem (separate crate: `aspen-fuse`)
- **aspen-net**: Service mesh daemon with SOCKS5 proxy, DNS, port forwarding (separate crate: `aspen-net`)
- **aspen-snix-bridge**: gRPC + nix-daemon bridge for snix-store ↔ Aspen cluster (separate crate: `aspen-snix-bridge`)
- **aspen-nix-cache-gateway**: HTTP binary cache gateway via nar-bridge (separate crate: `aspen-nix-cache-gateway`)
- **git-remote-aspen**: Git remote helper (requires `git-bridge` feature)
- **aspen-generate-schema**: Schema generator (requires `ci` feature)
- **aspen-cli token**: Capability token management (generate, inspect, delegate)

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

## Verus Formal Verification

Aspen uses [Verus](https://github.com/verus-lang/verus) to formally verify correctness of pure functions. Verus proves properties hold for ALL possible inputs, catching bugs testing cannot find.

### Two-File Architecture

```
crates/aspen-coordination/
├── src/verified/           # Production code (cargo-compiled)
│   ├── mod.rs              # Re-exports + module docs
│   └── lock.rs             # Pure functions: is_lock_expired, compute_deadline
└── verus/                  # Formal specs (Verus-verified)
    ├── lib.rs              # Module root + invariant documentation
    ├── lock_state_spec.rs  # State model + invariants
    └── acquire_spec.rs     # Operation pre/postconditions
```

**Why two files?** Production code compiles with standard `cargo build` (no Verus dependency). Verus specs are verified separately via `nix run .#verify-verus`. Zero runtime overhead.

### Writing Production Verified Functions

Location: `src/verified/*.rs`

```rust
//! Pure lock computation functions.
//! Formally verified - see `verus/lock_state_spec.rs` for proofs.

/// Check if a lock entry has expired.
/// A lock is expired if deadline_ms == 0 (released) or now > deadline.
#[inline]
pub fn is_lock_expired(deadline_ms: u64, now_ms: u64) -> bool {
    deadline_ms == 0 || now_ms > deadline_ms
}

/// Compute next fencing token. Uses saturating arithmetic.
#[inline]
pub fn compute_next_fencing_token(current_entry: Option<&LockEntry>) -> u64 {
    match current_entry {
        Some(entry) => entry.fencing_token.saturating_add(1),
        None => 1,
    }
}
```

**Rules for verified functions:**

- No I/O, no async, no system calls
- Time/randomness passed as explicit parameters
- Use saturating/checked arithmetic (`saturating_add`, not `+`)
- Keep functions short and focused (one logical operation)
- Add `#[inline]` for small functions

### Writing Verus Specifications

Location: `verus/*.rs`

```rust
use vstd::prelude::*;

verus! {
    // ========================================================================
    // State Model
    // ========================================================================

    pub struct LockState {
        pub entry: Option<LockEntrySpec>,
        pub current_time_ms: u64,
        pub max_fencing_token_issued: u64,
    }

    // ========================================================================
    // Spec Functions (mathematical definitions)
    // ========================================================================

    /// Check if lock is expired (spec version for proofs)
    pub open spec fn is_expired(entry: LockEntrySpec, current_time_ms: u64) -> bool {
        entry.deadline_ms == 0 || current_time_ms > entry.deadline_ms
    }

    // ========================================================================
    // Exec Functions (verified implementations)
    // ========================================================================

    /// Check if lock expired - ensures clause proves correctness
    pub fn is_lock_expired(deadline_ms: u64, now_ms: u64) -> (result: bool)
        ensures result == (deadline_ms == 0 || now_ms > deadline_ms)
    {
        deadline_ms == 0 || now_ms > deadline_ms
    }

    /// Compute next fencing token with monotonicity guarantee
    pub fn compute_next_fencing_token(current_token: Option<u64>) -> (result: u64)
        ensures
            result >= 1,
            current_token.is_some() ==> result >= current_token.unwrap()
    {
        match current_token {
            Some(token) => token.saturating_add(1).max(1),
            None => 1,
        }
    }

    // ========================================================================
    // Invariants
    // ========================================================================

    /// LOCK-1: Fencing token monotonicity
    pub open spec fn fencing_token_monotonic(pre: LockState, post: LockState) -> bool {
        post.max_fencing_token_issued >= pre.max_fencing_token_issued
    }

    /// Combined invariant
    pub open spec fn lock_invariant(state: LockState) -> bool {
        entry_token_bounded(state) &&
        state_ttl_valid(state) &&
        mutual_exclusion_holds(state)
    }

    // ========================================================================
    // Proofs
    // ========================================================================

    /// Proof: Initial state satisfies all invariants
    #[verifier(external_body)]
    pub proof fn initial_state_invariant(current_time_ms: u64)
        ensures lock_invariant(initial_lock_state(current_time_ms))
    {
        // SMT solver proves this automatically
    }
}
```

### Verus Function Types

| Type | Purpose | Example |
| ---- | ------- | ------- |
| `spec fn` | Mathematical specification, can use `forall`/`exists` | `spec fn is_expired(...) -> bool` |
| `exec fn` | Verified executable with `requires`/`ensures` | `fn compute_token(...) -> (r: u64) ensures r >= 1` |
| `proof fn` | Proof-only, guides SMT solver | `proof fn token_monotonic(...)` |
| `#[verifier(external_body)]` | Trust body, verify only ensures | Complex implementations |

### Invariant Documentation Pattern

Document all invariants in `verus/lib.rs`:

```rust
//! # Invariants Verified
//!
//! ## Distributed Lock
//!
//! 1. **LOCK-1: Fencing Token Monotonicity**: Tokens strictly increase
//! 2. **LOCK-2: Mutual Exclusion**: At most one holder at any time
//! 3. **LOCK-3: TTL Expiration Safety**: Expired locks are reacquirable
//!
//! ## Sequence Generator
//!
//! 1. **SEQ-1: Uniqueness**: No two next() calls return same value
//! 2. **SEQ-2: Monotonicity**: Each value strictly greater than previous
```

### Verification Commands

```bash
# Verify all specs (Core + Raft + Coordination)
nix run .#verify-verus

# Verify specific crate
nix run .#verify-verus core
nix run .#verify-verus raft
nix run .#verify-verus coordination

# Quick syntax check (no SMT solving)
nix run .#verify-verus -- quick

# Check ghost code compiles with cargo
nix run .#verus-inline-check
```

### Adding Verus Specs for New Features

1. **Create production function** in `src/verified/<feature>.rs`:

   ```rust
   pub fn compute_new_deadline(acquired_ms: u64, ttl_ms: u64) -> u64 {
       acquired_ms.saturating_add(ttl_ms)
   }
   ```

2. **Re-export from `src/verified/mod.rs`**:

   ```rust
   pub mod feature;
   pub use feature::compute_new_deadline;
   ```

3. **Create Verus spec** in `verus/<feature>_spec.rs`:

   ```rust
   verus! {
       pub fn compute_new_deadline(acquired_ms: u64, ttl_ms: u64) -> (result: u64)
           ensures
               acquired_ms as int + ttl_ms as int <= u64::MAX as int ==>
                   result == acquired_ms + ttl_ms,
               acquired_ms as int + ttl_ms as int > u64::MAX as int ==>
                   result == u64::MAX
       {
           acquired_ms.saturating_add(ttl_ms)
       }
   }
   ```

4. **Add to `verus/lib.rs`**:

   ```rust
   mod feature_spec;
   pub use feature_spec::compute_new_deadline;
   ```

5. **Run verification**:

   ```bash
   nix run .#verify-verus coordination
   ```

### Overflow Safety Helpers

Import from `verus/overflow_constants_spec.rs`:

```rust
verus! {
    // Check if addition is safe
    pub open spec fn can_add_u64(a: u64, b: u64) -> bool {
        a as int + b as int <= u64::MAX as int
    }

    // Use in requires clause
    pub fn safe_add(a: u64, b: u64) -> (result: u64)
        requires can_add_u64(a, b)
        ensures result == a + b
    {
        a + b
    }
}
```

### Current Verified Crates

| Crate | Verified Modules | Spec Files | Key Invariants |
| ----- | ---------------- | ---------- | -------------- |
| aspen-coordination | 20 | 27 | Locks, elections, queues, barriers, fencing |
| aspen-raft | 10 | 13 | Storage, chain integrity, batching, TTL |
| aspen-core | 1 | 6 | HLC, tuple encoding, directory ops |

### Trusted Axioms

Verus specs assume:

1. **CAS Linearizability**: Compare-and-swap is atomic (provided by Raft)
2. **Clock Monotonicity**: System time advances monotonically per node
3. **Bounded Clock Skew**: TTL accuracy bounded by max clock skew
4. **Type Bounds**: u64 in [0, 2^64-1], i64 in [-2^63, 2^63-1]

## Testing Philosophy

- **Integration over unit tests**: Real services, not mocks
- **Deterministic simulation**: `madsim` for reproducible distributed tests
- **Property-based testing**: `proptest` for edge cases, `bolero` for unified fuzz/property tests
- **Regression tests for every bug fix**: When fixing a bug or error, always write a test that reproduces it first, then fix the code to make the test pass. Don't just fix the bug — prevent it from coming back.
- **Never modify tests** unless explicitly asked

## Resource Bounds

Fixed limits in `crates/aspen-constants/src/` to prevent resource exhaustion:

- `MAX_BATCH_SIZE` = 1,000 entries
- `MAX_SCAN_RESULTS` = 10,000 keys
- `MAX_KEY_SIZE` = 1 KB, `MAX_VALUE_SIZE` = 1 MB
- `MAX_PEERS` = 1,000, `MAX_CONCURRENT_CONNECTIONS` = 500
- Timeouts: 5s connect, 2s stream, 10s read

## Git Commit Guidelines

- **Never add co-author attribution** (no `Co-Authored-By` trailers)
- Concise messages focusing on "why" not "what"
- Conventional commit style when appropriate

## OpenSpec Completion Guardrails

- Do not check a box in `tasks.md` unless the repo contains durable evidence for it.
- Add `verification.md` to the change directory once any task is checked. Use `openspec/templates/verification.md` as the template.
- Put saved command transcripts under `openspec/changes/.../evidence/`; do not rely on chat-only summaries or `/tmp` paths.
- Save a diff artifact in `evidence/` when checked tasks depend on source-file implementation claims.
- Every checked task in `tasks.md` must appear verbatim in `verification.md` with repo-relative evidence paths.
- If you cite `bash -n` or `scripts/openspec-preflight.sh` in your completion summary, save those transcripts as evidence too.
- Before `request_done_review` or claiming an OpenSpec task is complete, run `scripts/openspec-preflight.sh <change-dir-or-name>`.
- `scripts/openspec-preflight.sh` also checks that every `verification.md` `Changed file:` entry is currently modified/staged. For retroactive verification of already-landed code, keep `Implementation Evidence` limited to the new verification/evidence artifacts and cite committed source files under `Task Coverage` or saved diff artifacts.
- Stage newly created source files before `nix build` / `nix run`; untracked files can be excluded by the flake source filter and create false verification results.

## Important Notes

- Rust 2024 edition
- Backwards compatibility is not a concern; prioritize clean solutions
- Recursive `lib.fileset.toSource` entries must prune nested `.git`/`.direnv`/`target`/`result` directories before conversion. `lib.fileset.fileFilter` is not enough for this because descendants under excluded directory names still appear; use `lib.fileset.fromSource (lib.sources.cleanSourceWith { ... })`. Several crates accumulate their own `target/` trees; if `./crates` is included raw, `direnv`/`nix develop` can spend ages copying tens or hundreds of GB into the store during eval.
- All legacy `RaftActor` references have been removed; the codebase uses `RaftNode` throughout
- `openspec list --json` treats `openspec/changes/active/` as a change named `active`; inspect the dated directory inside `openspec/changes/active/` directly instead of assuming `active` is a real change
- Fresh trust joiners replay the historical `TrustInitialize` log even though they were not initial members. `apply_trust_initialize_in_txn()` must tolerate a missing local epoch-1 share and still persist the epoch metadata; otherwise the new node fatally shuts down before it can apply the later `TrustReconfiguration` entry that assigns its first real share.
- `ClientRpcRequest::InitClusterWithTrust { .. }` must stay classified as a bootstrap + rate-limit-exempt RPC in `crates/aspen-rpc-handlers/src/client.rs`; otherwise `aspen-cli cluster init --trust` is rejected before initialization and trust VM tests fail at startup.
- `IrohTrustShareClient::send_expunged()` must keep the QUIC stream alive until `send.stopped()` (or equivalent peer ack) completes; `send.finish()` alone lets the connection close before the removed node reads the expungement payload.
- Offline-expunged nodes may restart without issuing any trust traffic on their own. `crates/aspen-raft/src/trust_peer_probe.rs` must run synchronously during bootstrap, not as a delayed background task: `create_raft_node()` should wait for `ensure_startup_trust_peer_probe()` before returning so an `ExpungedByPeer` response persists the expunged marker before the node can continue booting.
- The startup trust probe must treat peer-reported `current_epoch` as authority. `probe_for_peer_expungement()` cannot infer health from locally stored digests alone; only peers whose `ShareResponse.current_epoch` matches the probed epoch count toward clearing startup.
- Secrets-at-rest epoch rotation must carry forward prior epoch keys into the new `SecretsEncryption` context until background re-encryption finishes; replacing the context with only the new epoch key breaks mixed-epoch reads during migration.
- `trust_current_epoch` is written only by `TrustReconfiguration`, not by `TrustInitialize`. Code that watches trust epochs must infer epoch 1 from persisted trust digests/shares when the metadata key is still absent.
- Redb-backed `RaftNode` tests cannot simulate a full in-process restart by reopening the same database path after `drop(node)`: openraft background tasks keep the Redb handle alive and `SharedRedbStorage::new()` returns `DatabaseAlreadyOpen`. Test startup-resume logic by creating a fresh provider/watcher on the existing node/storage state instead.
- `NodeBuilder::start()` always wires a trust share client when the `trust` feature is compiled (`crates/aspen-cluster/src/bootstrap/node/mod.rs::create_raft_node`). To verify "trust feature compiled but runtime trust not configured" behavior in `src/node/mod.rs`, test the secrets-service setup helper directly instead of relying on a bootstrapped node.
- `ClientRpcRequest` / `ClientRpcResponse` postcard discriminants are append-only compatibility contracts. New non-gated variants like `ExpungeNode` / `ExpungeNodeResult` must be appended at the end of the enum, not inserted into a domain section, or fixed discriminants like `Pong` / `Error` will shift and break older clients.
- In `crates/aspen-rpc-core`, the `forge` feature must pull in `blob` because `ClientProtocolContext` and `ForgeHandlerContext` expose `aspen_forge::ForgeNode<aspen_blob::IrohBlobStore, ...>` in public fields. If a feature-gated public type mentions another optional crate, model that dependency explicitly in `[features]`.
- `AppTypeConfig` can live in the leaf `aspen-raft-types` crate, but openraft trait impls whose `Self` type is external (for example `Arc<InMemoryStateMachine>`) still trip orphan rules. Keep shared config in the leaf crate and wrap external self types locally (`InMemoryStateMachineStore`) when you need trait impls in `aspen-raft`.
- `crates/aspen-cluster-types` is still part of the alloc-only `aspen-core` dependency path and `NodeAddress` must stay alloc-safe with iroh conversion helpers behind an opt-in feature. If `cargo tree -p aspen-core -e features` still shows `iroh` or `iroh-base` in the bare/default graph, check both `crates/aspen-cluster-types` and transitive consumers like `crates/aspen-traits`; disabling defaults on `aspen-core` alone is not enough if another direct dep re-enables `aspen-cluster-types` defaults.
- `scripts/check-aspen-core-no-std-boundary.py` plus `scripts/aspen-core-no-std-transitives.txt` are now the deterministic guardrail for the alloc-only graph. If the checker regresses on randomness again, inspect the vendored `uhlc` patch (`vendor/uhlc/`) first: Aspen relies on `uhlc` with `rand` optional so `aspen-hlc` can stay alloc-only without pulling `rand`, `rand_core`, or `getrandom` into `aspen-core --no-default-features`.
- Std helpers now live in `crates/aspen-core-shell/`. Shell consumers keep existing `aspen_core::*` import paths by aliasing that package under dependency key `aspen-core` (`aspen-core = { package = "aspen-core-shell", ... }`); alloc-only consumers still depend on real `crates/aspen-core/`.
- Vendored `openraft-macros` panicked on this toolchain because `openraft/macros/src/utils.rs::is_doc()` used debug assertions while scanning non-doc tokens. If `cargo check -p aspen-cluster` or `cargo check -p aspen-cli` explodes inside `#[since(...)]` again, inspect that helper first.
- Root `node-runtime` is now the minimal bootstrap slice (`aspen-cluster/bootstrap` only). Keep the old jobs/docs/hooks/federation combination behind the explicit `node-runtime-apps` bundle, and gate hook/federation-only code in `src/node/mod.rs` with their own features so `cargo check -p aspen --all-targets --no-default-features --features node-runtime` stays green.
