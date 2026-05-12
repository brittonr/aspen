## ADDED Requirements

### Requirement: Transport/RPC reusable defaults are evidence-backed

The `transport-rpc` family MUST prove that `aspen-transport` and `aspen-rpc-core` default feature graphs are reusable without Aspen node runtime, handler bundles, root app shells, cluster bootstrap, trust, sharding, or Raft compatibility crates unless those are behind named adapter/runtime features.

ID: transport-rpc-extraction.default-boundary-evidence

#### Scenario: Downstream fixtures compile

- GIVEN downstream fixtures that depend on `aspen-transport` and `aspen-rpc-core` default features
- WHEN `cargo metadata`, forbidden dependency scans, and `cargo check` run for both fixtures
- THEN the evidence MUST show the fixtures compile and do not depend on forbidden runtime crates.

#### Scenario: Runtime compatibility remains explicit

- GIVEN Aspen runtime consumers of transport/RPC APIs
- WHEN representative consumer `cargo check` commands run with explicit feature bundles
- THEN compatibility evidence MUST show the existing runtime paths still compile without broadening reusable defaults.

#### Scenario: Readiness checker gates promotion

- GIVEN transport/RPC manifest, policy, inventory, and evidence artifacts
- WHEN `scripts/check-crate-extraction-readiness.rs --candidate-family transport-rpc` runs
- THEN it MUST fail if required evidence is missing or default graphs leak forbidden dependencies, and pass before readiness is raised.
