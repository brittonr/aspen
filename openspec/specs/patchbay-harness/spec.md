## MODIFIED Requirements

### Requirement: Crate structure and dependencies

The `aspen-testing-patchbay` crate SHALL be a workspace member under `crates/aspen-testing-patchbay/` with `patchbay` as a dependency pinned to a specific git revision. It SHALL be gated behind a `patchbay` feature flag and SHALL NOT be included in the default build. Patchbay suites SHALL also register suite metadata through the shared Nickel manifest system so selection, prerequisite checks, and reporting use the same generated inventory as other harness layers.

#### Scenario: Crate compiles independently

- **WHEN** `cargo build -p aspen-testing-patchbay` is run
- **THEN** the crate compiles without errors and does not pull in non-test Aspen features (blob, ci, hooks, automerge)

#### Scenario: Feature flag gating

- **WHEN** a test uses `#[cfg(feature = "patchbay")]`
- **THEN** the test is excluded from default `cargo nextest run` and only runs when explicitly enabled

#### Scenario: Patchbay suites appear in generated inventory

- **WHEN** test metadata generation runs
- **THEN** each declared patchbay suite SHALL be emitted with layer `patchbay`, its prerequisites, and its execution target in the generated inventory
