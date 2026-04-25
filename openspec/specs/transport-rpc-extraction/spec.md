# transport-rpc-extraction Specification

## Purpose
TBD - created by archiving change split-transport-rpc-core. Update Purpose after archive.
## Requirements
### Requirement: Transport reusable surface avoids Aspen runtime shells
The reusable transport surface MUST expose Iroh/IRPC protocol-handler and stream/connection helpers without requiring root Aspen, cluster bootstrap, runtime auth, trust, sharding, Raft compatibility, handler registries, or domain services by default.

ID: transport-rpc-extraction.transport-default-avoids-runtime-shells

#### Scenario: Iroh adapter dependencies are documented exceptions
ID: transport-rpc-extraction.transport-default-avoids-runtime-shells.iroh-adapter-is-documented-exception

- **WHEN** the transport extraction manifest and policy are read
- **THEN** `iroh` and `irpc` SHALL be documented as allowed adapter-purpose dependencies for transport surfaces that explicitly own them
- **AND** Aspen app/runtime shell crates SHALL be forbidden from reusable defaults unless a named feature owns the dependency

#### Scenario: Runtime-only concerns are feature-gated or adapter-owned
ID: transport-rpc-extraction.transport-default-avoids-runtime-shells.runtime-concerns-feature-gated

- **WHEN** the default reusable transport graph is checked
- **THEN** it SHALL exclude `aspen-trust`, runtime `aspen-auth`, `aspen-sharding`, `aspen-raft`, root `aspen`, handler crates, cluster bootstrap, and binary shells
- **AND** any OpenRaft, sharding, trust, or auth integration SHALL be behind named features or adapter crates with explicit evidence

### Requirement: RPC core reusable surface avoids concrete Aspen service graph
The reusable RPC core surface MUST expose handler registry/dispatch abstractions without requiring concrete `RaftNode`, cluster bootstrap, transport adapter, coordination service, jobs, Forge, CI, hooks, blob, testing, or root Aspen app crates by default.

ID: transport-rpc-extraction.rpc-core-default-avoids-service-graph

#### Scenario: Handler registry compiles without concrete contexts
ID: transport-rpc-extraction.rpc-core-default-avoids-service-graph.registry-compiles-without-concrete-contexts

- **WHEN** `aspen-rpc-core` reusable feature sets are checked
- **THEN** handler registry, dispatch, operation metadata, metrics, and error plumbing SHALL compile without concrete Aspen domain contexts
- **AND** domain-specific context fields SHALL be hidden behind named feature gates or adapter crates

#### Scenario: Concrete domains are explicit opt-ins
ID: transport-rpc-extraction.rpc-core-default-avoids-service-graph.concrete-domains-are-explicit-opt-ins

- **WHEN** jobs, Forge, CI, hooks, blob, deploy, worker, cluster, or testing contexts are enabled
- **THEN** each domain SHALL be activated through a documented named feature or shell crate
- **AND** the feature graph SHALL identify which runtime dependencies become available

### Requirement: Downstream fixtures prove generic transport and RPC usage
Downstream-style fixtures MUST prove the reusable transport/RPC surfaces can be used without the Aspen node runtime.

ID: transport-rpc-extraction.downstream-fixtures

#### Scenario: Transport fixture uses generic protocol helpers
ID: transport-rpc-extraction.downstream-fixtures.transport-fixture-uses-generic-helpers

- **GIVEN** a downstream fixture manifest
- **WHEN** it compiles and records cargo metadata
- **THEN** it SHALL import the canonical transport surface directly
- **AND** it SHALL exercise protocol handler or stream/connection helper APIs
- **AND** metadata SHALL show root Aspen, cluster bootstrap, trust, sharding, Raft compatibility, handlers, and binary shells are absent

#### Scenario: RPC fixture uses registry without Aspen services
ID: transport-rpc-extraction.downstream-fixtures.rpc-fixture-uses-registry-without-services

- **GIVEN** a downstream fixture manifest
- **WHEN** it compiles and records cargo metadata
- **THEN** it SHALL construct or use the canonical handler registry/dispatch abstractions with a minimal test handler
- **AND** metadata SHALL show concrete Aspen service contexts are absent

### Requirement: Compatibility adapters preserve Aspen runtime behavior
Existing Aspen runtime consumers MUST continue to compile through explicit transport/RPC adapter features or compatibility crates.

ID: transport-rpc-extraction.compatibility-adapters-preserve-runtime

#### Scenario: Runtime consumers compile through documented features
ID: transport-rpc-extraction.compatibility-adapters-preserve-runtime.runtime-consumers-compile-through-features

- **WHEN** representative consumers such as `aspen-raft-network`, `aspen-raft`, `aspen-cluster`, `aspen-client`, `aspen-rpc-handlers`, and root node runtime are checked
- **THEN** they SHALL compile through documented runtime feature bundles or adapter crates
- **AND** evidence SHALL identify canonical reusable imports versus compatibility imports

### Requirement: Extraction inventory tracks transport/RPC family
The crate extraction inventory and policy MUST include transport/RPC candidates with readiness state, blocked reasons, owner, allowed adapter dependencies, forbidden runtime dependencies, representative consumers, and next action.

ID: transport-rpc-extraction.inventory-and-policy

#### Scenario: Readiness checker verifies staged split
ID: transport-rpc-extraction.inventory-and-policy.checker-verifies-staged-split

- **WHEN** `scripts/check-crate-extraction-readiness.rs --candidate-family transport-rpc` runs
- **THEN** it SHALL verify manifest fields, dependency exceptions, feature sets, representative consumers, downstream fixtures, and readiness labels
- **AND** it SHALL fail on unowned runtime dependencies, missing owners, invalid readiness states, or missing compatibility evidence

