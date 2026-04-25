# protocol-wire-extraction Specification

## Purpose
TBD - created by archiving change extract-protocol-wire-crates. Update Purpose after archive.
## Requirements
### Requirement: Protocol defaults avoid runtime shells
Protocol/wire crates MUST compile with reusable default feature sets that exclude root Aspen app crates, handler registries, node bootstrap, concrete transports, runtime auth shells, trust/secrets services, SQL engines, and UI/web/binary shells.

ID: protocol-wire-extraction.defaults-avoid-runtime-shells

#### Scenario: Client API default graph is reusable
ID: protocol-wire-extraction.defaults-avoid-runtime-shells.client-api-default-graph-is-reusable

- **WHEN** `cargo tree -p aspen-client-api --edges normal` and supported minimal feature graphs are checked
- **THEN** the graph SHALL exclude root `aspen`, `aspen-rpc-core`, `aspen-rpc-handlers`, `aspen-transport`, `aspen-raft`, `aspen-cluster`, runtime `aspen-auth`, concrete transport crates, UI/web/gateway crates, trust/secrets services, and SQL engines unless a documented named feature explicitly owns that dependency

#### Scenario: Domain protocol crates have no Aspen runtime deps
ID: protocol-wire-extraction.defaults-avoid-runtime-shells.domain-protocols-have-no-runtime-deps

- **WHEN** `cargo tree` is checked for `aspen-forge-protocol`, `aspen-jobs-protocol`, and `aspen-coordination-protocol`
- **THEN** their default reusable graphs SHALL contain only serialization libraries and documented leaf/protocol dependencies
- **AND** they SHALL exclude handler registries, root Aspen, concrete transport, and binary shell crates

### Requirement: Wire compatibility is append-only and reviewable
Public wire enums and schema-critical protocol types MUST have durable compatibility evidence proving existing postcard discriminants and encoded forms remain stable.

ID: protocol-wire-extraction.wire-compatibility-reviewable

#### Scenario: Existing enum discriminants stay stable
ID: protocol-wire-extraction.wire-compatibility-reviewable.enum-discriminants-stay-stable

- **GIVEN** public request/response enums such as `ClientRpcRequest` and `ClientRpcResponse`
- **WHEN** compatibility tests encode representative existing variants
- **THEN** encoded discriminants SHALL match saved baselines
- **AND** new variants SHALL be appended after existing variants rather than inserted before them

#### Scenario: Golden artifacts are saved for review
ID: protocol-wire-extraction.wire-compatibility-reviewable.golden-artifacts-are-saved

- **WHEN** protocol compatibility checks run for the family
- **THEN** saved evidence SHALL include command transcripts and golden/snapshot artifacts or hashes sufficient for review
- **AND** `verification.md` SHALL identify which artifacts prove each compatibility claim before any task is checked

### Requirement: Portable auth and ticket references avoid runtime shells
Wire crates that expose auth, capability, hook, or ticket types MUST depend on portable leaf crates rather than runtime verifier/builder shells by default.

ID: protocol-wire-extraction.portable-auth-ticket-types

#### Scenario: Auth feature uses auth core
ID: protocol-wire-extraction.portable-auth-ticket-types.auth-feature-uses-auth-core

- **WHEN** auth-related protocol features are enabled
- **THEN** portable capability/token wire types SHALL come from `aspen-auth-core` or other leaf crates
- **AND** runtime `aspen-auth` verifier, HMAC, and revocation storage dependencies SHALL remain outside default reusable protocol graphs

#### Scenario: Hook tickets stay in ticket crate
ID: protocol-wire-extraction.portable-auth-ticket-types.hook-tickets-use-ticket-crate

- **WHEN** hook-related wire types need `AspenHookTicket`
- **THEN** they SHALL depend on `aspen-hooks-ticket` or a protocol/leaf re-export
- **AND** they SHALL NOT require `aspen-hooks` runtime handlers by default

### Requirement: Downstream serialization fixture uses canonical protocol crates
A downstream-style fixture MUST prove protocol crates can be used for encode/decode workflows without root Aspen, handlers, runtime transports, or compatibility re-exports.

ID: protocol-wire-extraction.downstream-serialization-proof

#### Scenario: Fixture encodes and decodes canonical wire types
ID: protocol-wire-extraction.downstream-serialization-proof.fixture-serializes-canonical-types

- **GIVEN** an in-tree downstream fixture manifest
- **WHEN** the fixture compiles and runs serialization checks
- **THEN** it SHALL import canonical protocol crates directly
- **AND** it SHALL encode and decode at least one client API type and one domain protocol type
- **AND** metadata SHALL prove forbidden runtime/app crates are absent

### Requirement: Extraction inventory tracks protocol family
The crate extraction inventory and policy MUST include the protocol/wire family with per-crate readiness state, owner, feature contract, compatibility rails, and next action.

ID: protocol-wire-extraction.inventory-and-policy

#### Scenario: Policy checker verifies protocol family
ID: protocol-wire-extraction.inventory-and-policy.policy-checker-verifies-family

- **WHEN** `scripts/check-crate-extraction-readiness.rs --candidate-family protocol-wire` is run
- **THEN** it SHALL validate manifest presence, owner fields, readiness labels, dependency exceptions, representative consumers, and forbidden dependency boundaries for all protocol/wire candidates
