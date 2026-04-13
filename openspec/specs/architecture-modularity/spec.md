# Architecture Modularity Specification

## Purpose

Keep client RPC ownership, handler capability wiring, bootstrap phases, shared protocol types, and feature bundles aligned with narrow cross-crate boundaries. Contributors should be able to change one domain without reopening unrelated global edit hotspots or dragging in broad app bundles just to compile a core node slice.

## Requirements

### Requirement: Domain-owned client RPC contracts

The system SHALL define client RPC ownership at domain boundaries instead of concentrating optional-app protocol metadata in one global edit hotspot.

#### Scenario: Adding a forge operation does not require a global metadata edit

- **GIVEN** a contributor adds a new Forge-specific client RPC operation
- **WHEN** the operation is wired into the client protocol
- **THEN** the payload type and request metadata SHALL be owned by Forge-domain code
- **AND** the contributor SHALL NOT need to extend a central cross-domain metadata match for unrelated apps
- **AND** existing wire-compatibility checks SHALL continue to pass

#### Scenario: Stable outer envelope during migration

- **GIVEN** Aspen is migrating away from a mega-enum ownership model
- **WHEN** a refactor slice moves one domain's metadata behind a local boundary
- **THEN** the client protocol SHALL preserve the documented outer framing and versioning behavior
- **AND** golden tests SHALL detect any unintended wire drift

### Requirement: Capability-scoped handler dependencies

The system SHALL inject handler dependencies through declared capabilities or bounded subcontexts instead of one shared global context struct.

#### Scenario: Handler registration declares required capabilities

- **GIVEN** a handler needs key-value access and metrics access but no Forge or CI services
- **WHEN** the handler is registered
- **THEN** its factory or installer SHALL declare only those required capabilities
- **AND** composition SHALL fail with an explicit error if a declared capability is missing
- **AND** the handler SHALL NOT receive unrelated mutable or optional subsystem handles

#### Scenario: Test builders stay narrow

- **GIVEN** a unit test for one handler domain
- **WHEN** the test constructs the handler environment
- **THEN** the test builder SHALL be able to provide only the capabilities used by that handler
- **AND** unrelated subsystems SHALL remain absent without breaking the test harness

### Requirement: Phase-composed node bootstrap

The system SHALL compose node startup as bounded phases with explicit outputs so minimal node slices can build and start without large unrelated feature conjunctions.

#### Scenario: Minimal node slice builds without app bundles

- **GIVEN** a build that enables storage, transport, and core client RPC only
- **WHEN** `NodeBuilder::start()` or the equivalent bootstrap path is compiled for that slice
- **THEN** the build SHALL succeed without requiring blob, docs, jobs, hooks, or federation together
- **AND** startup SHALL execute only the enabled phases

#### Scenario: App installer consumes prior phase outputs

- **GIVEN** an optional subsystem such as Forge or CI is enabled
- **WHEN** its installer runs during startup
- **THEN** it SHALL consume typed outputs from earlier bootstrap phases
- **AND** it SHALL register its own RPC handlers or services without editing one monolithic setup function for unrelated domains

### Requirement: Shared consensus and transport types have one source of truth

Any protocol type shared by the consensus and transport layers SHALL live behind one leaf crate boundary and SHALL NOT require `unsafe` structural bridging.

#### Scenario: Transport uses the shared Raft type directly

- **GIVEN** the transport layer needs the Raft application type configuration
- **WHEN** it is compiled with the consensus layer
- **THEN** both crates SHALL import the same shared type definition from a leaf boundary
- **AND** startup code SHALL NOT use `unsafe transmute` to reinterpret one crate's type as another

### Requirement: Feature bundles are explicit and bounded

The build feature graph SHALL distinguish leaf features from convenience bundles so enabling one subsystem does not silently pull unrelated app domains without an explicit bundle name.

#### Scenario: Leaf feature enables only direct prerequisites

- **GIVEN** a contributor enables a leaf feature for one subsystem
- **WHEN** Cargo resolves the feature graph
- **THEN** the enabled dependencies SHALL be limited to that subsystem and its direct prerequisites
- **AND** unrelated app domains SHALL remain disabled unless a named bundle requests them

#### Scenario: Bundle names document cross-app composition

- **GIVEN** a build profile requires multiple app domains together
- **WHEN** the profile is expressed through Cargo features
- **THEN** the composition SHALL use an explicitly named bundle feature
- **AND** documentation and compile-slice tests SHALL identify what that bundle includes
