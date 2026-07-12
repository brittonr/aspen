# Node Replication Pilot Specification Delta

## ADDED Requirements

### Requirement: The compatibility probe consumes Octet toolchain packages [r[molten.node_replication_pilot.octet_toolchain_consumption]]

The node-replication compatibility probe MUST consume its Verus profile and production verifier package from an exact published Octet revision. The pilot MUST NOT independently reconstruct the production verifier or its Rust toolchain from copied release metadata. The decision evidence MUST bind the Octet revision, profile artifact identity, and verifier package output.

#### Scenario: Central verifier package reproduces the blocked probe

- GIVEN the pinned Octet revision exposes the reviewed profile and production verifier packages
- WHEN the compatibility probe runs with and without the required feature flag
- THEN it MUST execute the Octet verifier wrapper and preserve distinct bounded diagnostics
- AND its decision MUST bind the Octet revision, profile digest, and verifier output

#### Scenario: Central profile drift blocks the probe

- GIVEN the required Octet package is absent or its profile differs from the pilot's reviewed admission values
- WHEN the compatibility probe evaluates or executes
- THEN the pilot MUST fail before emitting compatibility evidence
- AND it MUST NOT reconstruct a local verifier fallback

#### Scenario: Package provenance does not authorize runtime adoption

- GIVEN the probe uses exact Octet packages but still encounters an unsupported feature or verifier internal error
- WHEN promotion eligibility is computed
- THEN runtime dependency status MUST remain denied
- AND package identity MUST NOT discharge trusted boundaries, concurrency testing, NUMA benchmarking, rollback, or distributed-system non-claims
