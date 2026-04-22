## ADDED Requirements

### Requirement: Transport-neutral Raft membership metadata
ID: transport.transport-neutral-raft-membership-metadata

The system SHALL store transport-neutral `NodeAddress` values in Raft membership metadata and trust membership payloads rather than concrete runtime iroh address types. Runtime shells SHALL convert those values into `iroh::EndpointAddr` only when they need to open connections or populate runtime-only registries.

#### Scenario: Membership metadata stays transport-neutral at rest
ID: transport.transport-neutral-raft-membership-metadata.membership-metadata-stays-transport-neutral-at-rest

- **WHEN** Aspen persists `RaftMemberInfo` or trust membership payload data in `aspen-raft-types`
- **THEN** the stored representation SHALL contain `NodeAddress` data
- **AND** `crates/aspen-raft-types` SHALL NOT depend directly on `iroh`

#### Scenario: Runtime conversion failure is surfaced explicitly
ID: transport.transport-neutral-raft-membership-metadata.runtime-conversion-failure-is-surfaced-explicitly

- **GIVEN** runtime code needs to turn stored membership metadata into an `iroh::EndpointAddr`
- **WHEN** the stored endpoint id or transport address set is invalid
- **THEN** the runtime path SHALL log a warning that identifies the failing member
- **AND** the operation SHALL return its normal retryable/unavailable outcome instead of silently pretending the target never existed

#### Scenario: Default member metadata stays parseable for runtime helpers
ID: transport.transport-neutral-raft-membership-metadata.default-member-metadata-stays-parseable-for-runtime-helpers

- **WHEN** test helpers or `openraft` utilities construct `RaftMemberInfo::default()`
- **THEN** the default endpoint id SHALL remain parseable by runtime membership, relay, and blob-topology helper paths
- **AND** default metadata SHALL remain deterministic across runs
