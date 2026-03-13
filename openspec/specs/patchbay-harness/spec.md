## ADDED Requirements

### Requirement: Crate structure and dependencies

The `aspen-testing-patchbay` crate SHALL be a workspace member under `crates/aspen-testing-patchbay/` with `patchbay` as a dependency pinned to a specific git revision. It SHALL be gated behind a `patchbay` feature flag and SHALL NOT be included in the default build.

#### Scenario: Crate compiles independently

- **WHEN** `cargo build -p aspen-testing-patchbay` is run
- **THEN** the crate compiles without errors and does not pull in non-test Aspen features (blob, ci, hooks, automerge)

#### Scenario: Feature flag gating

- **WHEN** a test uses `#[cfg(feature = "patchbay")]`
- **THEN** the test is excluded from default `cargo nextest run` and only runs when explicitly enabled

### Requirement: PatchbayHarness spawns nodes in network namespaces

The harness SHALL provide a `PatchbayHarness` struct that creates a patchbay `Lab`, adds routers and devices, and spawns minimal Aspen nodes (iroh endpoint + Raft + KV store) inside device namespaces via `device.spawn()`.

#### Scenario: Single node boots in namespace

- **WHEN** `PatchbayHarness` creates one device behind a public router and spawns an Aspen node
- **THEN** the node's iroh endpoint binds inside the device namespace and the node is reachable only through the simulated network

#### Scenario: Three nodes form a Raft cluster

- **WHEN** three nodes are spawned across devices and `init_cluster()` is called
- **THEN** a Raft leader is elected within 30 seconds and all three nodes report a consistent membership set

### Requirement: Topology presets

The harness SHALL provide builder methods for common topologies: `three_node_public()`, `three_node_home_nat()`, `mixed_nat()`, and `two_region()`.

#### Scenario: Public topology baseline

- **WHEN** `PatchbayHarness::three_node_public()` is called
- **THEN** three devices are created behind a single `RouterPreset::Public` router with no NAT and direct connectivity between all nodes

#### Scenario: Two-region topology with latency

- **WHEN** `PatchbayHarness::two_region(eu_nodes: 2, us_nodes: 1, latency_ms: 80)` is called
- **THEN** two regions are created with 80ms inter-region latency, EU nodes are behind an EU router, and the US node is behind a US router

### Requirement: Runtime userns detection and graceful skip

The harness SHALL check for unprivileged user namespace support at test startup. If unavailable, tests SHALL skip with a clear diagnostic message rather than failing.

#### Scenario: Userns unavailable

- **WHEN** `kernel.unprivileged_userns_clone` is 0 or AppArmor restricts userns
- **THEN** the test prints "skipping: unprivileged user namespaces not available" and returns success

#### Scenario: Userns available

- **WHEN** unprivileged user namespaces are functional
- **THEN** the harness proceeds with `patchbay::init_userns()` and `Lab::new()`

### Requirement: Node handle access for assertions

The harness SHALL expose handles to each spawned node's `RaftNode` (or a proxy) so tests can assert on internal state: current leader, applied index, membership, and KV reads.

#### Scenario: Read KV value from specific node

- **WHEN** a test writes a KV pair through node 0 and reads it from node 2's handle
- **THEN** node 2 returns the written value after replication completes

#### Scenario: Check leader on all nodes

- **WHEN** a test calls `harness.check_leader()` after cluster formation
- **THEN** all nodes agree on the same leader ID
