## Context

Aspen has three testing layers today:

1. **In-memory** (`AspenRouter`): Fast, deterministic, fake networking. Tests Raft logic but not real transport.
2. **Madsim** (`AspenRaftTester`): Deterministic simulation with buggify-style fault injection. Simulates network but doesn't exercise real QUIC/iroh.
3. **NixOS VM tests** (44 tests in `nix/tests/`): Full system tests with real networking but coarse fault injection (iptables). Heavy — each test builds a NixOS VM.

The gap: no tests run real iroh QUIC connections through realistic NAT topologies without the overhead of full VMs. Patchbay fills this gap — it uses Linux network namespaces (unprivileged, O(ms) setup, cleanup-on-drop) to create real network topologies where real iroh endpoints talk through simulated routers with configurable NAT, firewalls, and link conditions.

Current `aspen-testing-network` has `fault_injection.rs` (iptables-based partitions, tc-based latency/loss) and `vm_manager.rs` (Cloud Hypervisor VMs). These work but require either root (iptables) or heavy infrastructure (VMs). Patchbay runs unprivileged and gives finer control.

## Goals / Non-Goals

**Goals:**

- Introduce a patchbay-based test harness that spawns real Aspen nodes (iroh endpoint + Raft) inside network namespaces
- Test Raft cluster formation across Home NAT, Corporate NAT, CGNAT, and Public topologies
- Test KV replication under link degradation (latency, packet loss, bandwidth limits)
- Test partition/heal recovery with patchbay's `break_region_link` / `restore_region_link`
- Run these tests in CI without root privileges
- Keep the harness reusable for future topology scenarios (federation, proxy, multi-region)

**Non-Goals:**

- Replacing madsim — madsim remains the tool for deterministic replay and buggify testing
- Replacing NixOS VM tests — those test full system integration (systemd, config, boot)
- Testing iroh internals — patchbay tests Aspen's behavior over iroh, not iroh itself
- GUI or devtools UI integration — patchbay's web UI is nice but out of scope for initial integration
- macOS support — patchbay requires Linux namespaces

## Decisions

### 1. New crate: `aspen-testing-patchbay`

**Decision**: Create a standalone crate rather than adding patchbay to `aspen-testing-network`.

**Rationale**: `aspen-testing-network` uses iptables and Cloud Hypervisor — different dependencies and different privilege model. Patchbay is unprivileged and namespace-based. Keeping it separate means:

- Clear feature boundary: `patchbay` feature flag only pulls in what's needed
- Independent CI gating: can skip patchbay tests on systems without userns support
- Clean dependency tree: patchbay brings its own nftables/tc wrappers

**Alternative considered**: Adding a `patchbay` feature to `aspen-testing-network`. Rejected because the two approaches have fundamentally different runtime requirements.

### 2. Node spawning via `device.spawn()` with in-process Aspen

**Decision**: Run Aspen nodes as async tasks inside patchbay device namespaces using `device.spawn()`, not as separate OS processes.

**Rationale**: `device.spawn()` runs an async closure on the device's per-namespace tokio runtime. This means:

- The iroh endpoint binds inside the namespace — sees the namespace's network stack
- No need to build and distribute a separate binary
- Direct access to `RaftNode` handles for assertions (leader checks, applied index, KV reads)
- Faster test setup — no process spawning or IPC

**Alternative considered**: `device.spawn_command()` with `cargo run` or a prebuilt binary. Rejected because it's slower and harder to assert on internal state.

### 3. Topology presets via builder API

**Decision**: Define common test topologies as builder methods on the harness:

- `three_node_public()`: 3 nodes behind a public router (baseline)
- `three_node_home_nat()`: 3 nodes behind a home NAT router
- `mixed_nat(public: N, home: N, corporate: N)`: mixed topology
- `two_region(eu_nodes: N, us_nodes: N, latency_ms: u32)`: cross-region

**Rationale**: Most tests need the same few topologies. Builders reduce boilerplate while still allowing custom topologies via the raw patchbay API.

### 4. Test gating via `#[cfg(feature = "patchbay")]` and nextest filter

**Decision**: Gate patchbay tests behind a cargo feature and a nextest filter expression. Tests check for userns support at runtime and skip gracefully if unavailable.

**Rationale**: Not all CI runners support unprivileged user namespaces (e.g., some Docker configurations). Runtime detection + skip is more robust than compile-time gating alone.

### 5. Stripped-down node bootstrap for test speed

**Decision**: The harness boots minimal Aspen nodes — iroh endpoint + Raft + KV store only. No blob storage, no CI agent, no hooks, no automerge.

**Rationale**: Patchbay tests target networking and consensus behavior. Loading the full feature set would slow test startup and add unrelated failure modes. Tests that need additional features can opt in.

## Risks / Trade-offs

**[Namespace support varies across CI environments]** → Runtime detection: check `kernel.unprivileged_userns_clone` sysctl and skip tests with clear message if unavailable. Document required CI runner configuration.

**[Patchbay is young (10 stars, 241 commits)]** → It's from n0-computer and actively maintained. Pin to a specific git revision. If it breaks, the existing madsim and NixOS VM tests still provide coverage.

**[In-process nodes share memory — not fully isolated]** → Each node gets its own namespace and tokio runtime via `device.spawn()`, but they share the process address space. This is fine for networking tests. For full isolation (disk corruption, OOM), use NixOS VM tests.

**[Test flakiness from real networking]** → Use patchbay's deterministic link conditions (fixed latency, not jitter) for most tests. Add retry with different seeds only for chaos/stress tests. Set generous timeouts (iroh relay fallback can take seconds).

**[Patchbay requires nft and tc in PATH]** → Add these to the nix devshell. They're standard Linux networking tools.
