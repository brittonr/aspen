# test-harness-runtime Specification

## Purpose

Defines the Test Harness Runtime capability requirements preserved by Aspen's archived OpenSpec records, including shared harness facade across test layers, wait-driven readiness helpers cover critical test flows.
## Requirements
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

### Requirement: Broader Quick Confidence Rail [r[test-harness-runtime.quick-confidence-rail]]

Aspen MUST provide a broader quick confidence rail that composes selected local checks into one bounded operator command or check profile with a structured summary and explicit non-proof boundaries.

#### Scenario: Quick rail runs selected local checks [r[test-harness-runtime.quick-confidence-rail.selected-checks]]

- GIVEN a developer wants broader local confidence without running full dogfood or gated VM proofs
- WHEN the quick confidence rail runs
- THEN it SHALL execute a documented set of local checks such as quick Rust tests, harness metadata checks, relevant docs guardrails, and OpenSpec validation
- AND it SHALL report each included check with pass/fail status

#### Scenario: Quick rail reports skipped gated proofs [r[test-harness-runtime.quick-confidence-rail.skipped-gated-proofs]]

- GIVEN gated runtime-host proofs require KVM, Uhyve, Hyperlight, or other expensive environment support
- WHEN the quick confidence rail completes without running those proofs
- THEN its summary SHALL explicitly state that those gated proofs were skipped and SHALL NOT claim runtime-host acceptance from the quick rail alone

#### Scenario: Quick rail failure is actionable [r[test-harness-runtime.quick-confidence-rail.actionable-failure]]

- GIVEN one selected check fails
- WHEN the rail reports the result
- THEN it SHALL identify the failing check name, command or check profile, exit status, and next diagnostic pointer without hiding earlier successful checks

### Requirement: Stock cluster VM checks are durable evidence rails [r[test-harness-runtime.stock-cluster-vm-checks]]

Aspen MUST keep the stock cluster VM checks runnable as focused operator evidence targets for repository-supported clustering behavior.

#### Scenario: MicroVM cluster check reaches product assertions [r[test-harness-runtime.stock-cluster-vm-checks.microvm-runs]]

- GIVEN an operator runs `nix build --impure .#checks.x86_64-linux.microvm-cluster-test --no-link -L` on a host with the required VM capabilities
- WHEN the check builds its closure successfully
- THEN it SHALL execute the VM test script instead of failing before VM execution due dependency or packaging drift
- AND it SHALL assert the intended multi-node cluster and AspenFs connection behavior

#### Scenario: Multi-node cluster check exposes a core cluster proof boundary [r[test-harness-runtime.stock-cluster-vm-checks.core-boundary]]

- GIVEN an operator runs `nix build --impure .#checks.x86_64-linux.multi-node-cluster-test --no-link -L`
- WHEN node startup, cluster initialization, learner addition, and voter promotion succeed
- THEN the check SHALL emit or record an explicit core-cluster proof boundary before optional plugin/forge assertions run
- AND a later optional plugin/forge failure SHALL NOT be reported as failure to boot or form the Raft/Iroh cluster

#### Scenario: Focused VM failures are classified by boundary [r[test-harness-runtime.stock-cluster-vm-checks.failure-classification]]

- GIVEN a stock cluster VM check fails
- WHEN evidence is reported or preserved
- THEN the failure SHALL be classified as one of build closure, host VM capability, VM boot/service readiness, cluster formation, optional plugin/forge fixture, or post-formation product assertion
- AND cluster tickets or equivalent secrets SHALL be redacted from committed evidence
