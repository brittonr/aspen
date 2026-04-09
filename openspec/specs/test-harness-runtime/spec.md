## ADDED Requirements

### Requirement: Shared harness facade across test layers

The harness SHALL expose shared cluster lifecycle and assertion APIs through `crates/aspen-testing*` for integration suites that boot Aspen clusters. Real-network, madsim, patchbay, and VM-oriented helpers SHALL share common operations for cluster setup, readiness, and fault control so suites can move between layers without re-implementing bootstrap logic.

#### Scenario: Real-cluster integration tests use the shared facade

- **WHEN** a real-network integration suite needs to start a cluster and obtain client handles
- **THEN** it SHALL use the shared facade exported from `aspen-testing` instead of maintaining a separate standalone bootstrap implementation under `tests/support`

#### Scenario: Layer adapters preserve common operations

- **WHEN** a suite is promoted from one layer to another
- **THEN** common operations such as `init_cluster`, `wait_for_leader`, `wait_for_replication`, and fault injection controls SHALL keep consistent semantics across the supported harness layers

### Requirement: Wait-driven readiness helpers cover critical test flows

The harness SHALL provide bounded wait helpers for common readiness and convergence conditions in Rust and NixOS VM suites. Critical test flows SHALL express the condition being awaited instead of relying on unconditional fixed sleeps when they are waiting on observable state changes.

#### Scenario: Rust integration tests wait on named conditions

- **WHEN** a Rust integration suite needs to observe leader election, replication, job completion, or cluster health
- **THEN** it SHALL use a bounded wait helper that names the awaited condition instead of an unconditional fixed sleep

#### Scenario: VM suites share reusable readiness helpers

- **WHEN** a NixOS VM suite needs service, socket, cluster, or job readiness
- **THEN** it SHALL use shared helper functions so the readiness condition and timeout policy are reusable across VM suites
