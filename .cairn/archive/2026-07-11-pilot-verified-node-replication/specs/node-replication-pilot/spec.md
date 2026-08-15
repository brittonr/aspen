# Verified Node Replication Pilot Specification

## ADDED Requirements

### Requirement: Typed pilot profile
r[molten.node_replication_pilot.profile] Molten MUST define a typed pilot profile that binds the exact verified-node-replication source, Verus submodule, fixed-output hashes, license, local-NUMA scope, resource bounds, promotion criteria, and explicit non-claims.

#### Scenario: Valid local pilot profile passes
- GIVEN an exact immutable source profile with non-empty promotion criteria and non-claims
- WHEN profile validation runs
- THEN the profile MUST be accepted as pilot configuration only.

### Requirement: Pilot validation rejects distributed overclaims
r[molten.node_replication_pilot.validation] The pilot MUST reject profiles that describe node replication as network replication, distributed consensus, fabric consistency, or proof of Molten runtime correctness.

#### Scenario: Distributed-replication wording fails
- GIVEN a profile requests distributed-consistency authority from the upstream crate
- WHEN validation runs
- THEN it MUST fail before source or verifier execution.

### Requirement: Exact source identity
r[molten.node_replication_pilot.source_identity] Source admission MUST use exact revisions and fixed-output hashes and MUST validate the MIT license, crate manifest, proof root, and example sentinels.

#### Scenario: Missing source sentinel blocks pilot
- GIVEN a fetched source lacks any required sentinel
- WHEN source admission runs
- THEN the pilot MUST be blocked before verifier execution.

### Requirement: Bounded verifier compatibility probe
r[molten.node_replication_pilot.verifier_probe] The pilot SHOULD run the reviewed Octet production Verus with explicit argv, bounded logs, and the required feature flags. Success, verifier failure, internal error, timeout, and unsupported feature MUST produce distinct bounded decisions.

#### Scenario: Verifier internal error blocks adoption
- GIVEN the pinned source triggers an internal verifier error
- WHEN the compatibility probe runs
- THEN the pilot MUST report `blocked-verifier`
- AND runtime dependency promotion MUST remain denied.

### Requirement: Trusted proof boundaries remain visible
r[molten.node_replication_pilot.trusted_boundary] The pilot MUST inventory upstream trusted proof markers and MUST NOT describe a passing verifier run as independently proving trusted theorem or trait bodies.

#### Scenario: Trusted top-level theorem is recorded
- GIVEN the source contains a trusted refinement theorem
- WHEN the trusted-boundary audit runs
- THEN the marker and source location MUST appear in the pilot evidence and non-claims.

### Requirement: Promotion is fail closed
r[molten.node_replication_pilot.promotion] Molten MUST NOT add verified-node-replication to runtime dependencies until verifier compatibility, trusted-boundary review, API scope, positive and negative concurrency tests, NUMA benchmark bounds, rollback, and Octet/provenance evidence all pass.

#### Scenario: One unsatisfied criterion denies promotion
- GIVEN any required promotion criterion is missing or blocked
- WHEN the pilot decision is computed
- THEN the decision MUST deny runtime promotion with an actionable blocker.

### Requirement: Local NUMA boundary
r[molten.node_replication_pilot.boundary] Documentation MUST identify node replication as a local multicore/NUMA data-structure technique and MUST keep it distinct from Iroh transport, distributed consistency, Raft, fabric membership, and network replication.

#### Scenario: Reference remains scoped
- GIVEN the upstream repository is cited in Molten documentation
- WHEN a reader inspects the reference
- THEN the text MUST state the local scope and proof-transfer non-claim.

### Requirement: Final validation
r[molten.node_replication_pilot.final_validation] Positive and negative profile fixtures, source admission, verifier probe, trusted-boundary audit, promotion denial, and Cairn validation MUST complete before archive.

#### Scenario: Blocked pilot is a valid result
- GIVEN source admission passes but verifier compatibility remains blocked
- WHEN final validation runs
- THEN the change MAY complete with a deterministic blocker receipt
- AND MUST NOT synthesize runtime adoption or benchmark success.
