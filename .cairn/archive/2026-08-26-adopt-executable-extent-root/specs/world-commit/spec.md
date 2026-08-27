# Molten World Commit Specification Delta

## Purpose

Adopt immutable executable extents as an optional world code-root profile while preserving build, mapping, runtime, and authority boundaries.

## ADDED Requirements

### Requirement: Executable extent dependencies are pinned and reviewed

r[molten.world_extents.dependency] Molten MUST consume the executable-extent mechanism and Mantle producer contract from immutable reviewed revisions. Adoption MUST verify package identity, license, API boundary, Octet gate, positive and negative vectors, and source parity before use.

#### Scenario: Dependency uses a sibling path

- GIVEN Molten configuration references an ambient sibling checkout instead of an immutable source
- WHEN dependency admission runs
- THEN adoption MUST fail before build or mapping

### Requirement: Code and mapping identities remain nominally distinct

r[molten.world_extents.identity_domains] Molten MUST use separate types for semantic code, built artifact, extent manifest, individual extent, and live mapping identities. A value from one domain MUST NOT substitute for another.

#### Scenario: Artifact digest is supplied as a mapping identity

- GIVEN a valid build artifact digest appears in a live mapping field
- WHEN extent admission runs
- THEN Molten MUST reject the value as an identity-domain mismatch

### Requirement: Extent world roots bind exact producer and runtime cohorts

r[molten.world_extents.profile] An extent code-root profile MUST bind the source artifact, Mantle bundle, target triple, executable format, ABI cohort, page profile, ordered extent descriptors, complete closure, runtime cohort, and policy identity.

#### Scenario: Complete extent profile is inspected

- GIVEN every bound object is available and exact profile facts match
- WHEN world closure validation runs
- THEN the extent root MAY enter mapping admission

#### Scenario: Ordinary artifact is supplied to an extent-required policy

- GIVEN bytes are a valid ordinary artifact but no admitted extent manifest exists
- WHEN extent-required admission runs
- THEN Molten MUST deny the profile without silent fallback

### Requirement: Mapping admission remeasures exact bytes and layout

r[molten.world_extents.admission] Molten MUST remeasure every extent and validate digest, length, offset, alignment, overlap, target, format, ABI, permission, page profile, and complete closure before mapping.

#### Scenario: Producer receipt passes but one extent changed

- GIVEN a valid producer receipt names bytes that no longer match the extent digest
- WHEN consumer remeasurement runs
- THEN Molten MUST reject mapping despite producer success

### Requirement: W^X is an explicit state transition

r[molten.world_extents.wx] Molten MUST use the shared pure W^X transition contract. No admitted state or transition MAY make the same extent writable and executable at the same time.

#### Scenario: Sealed extent becomes executable

- GIVEN exact bytes are sealed read-only and fresh read-back passes
- WHEN current runtime and execution authority admit activation
- THEN the shell MAY map the extent executable and read-only

#### Scenario: Adapter requests writable executable memory

- GIVEN a mapping request includes write and execute permission together
- WHEN mapping planning runs
- THEN Molten MUST reject the request before host mapping effects

### Requirement: Materialization and mapping are capability-relative

r[molten.world_extents.materialization] The shell MUST materialize, verify, seal, and map through admitted capability handles. It MUST NOT verify one path and reopen an ambient path for mapping.

#### Scenario: Path target is substituted after verification

- GIVEN an ambient path changes after bytes were verified
- WHEN mapping uses the already verified handle
- THEN the substituted path MUST NOT change the mapped object

### Requirement: Extent validity does not grant activation

r[molten.world_extents.activation] Molten MUST recheck current artifact, runtime, resource, policy, and execution authority before activation. Extent validity or possession MUST NOT grant execution.

#### Scenario: Valid extent lacks current execution authority

- GIVEN manifest and mapping admission pass but current policy denies execution
- WHEN activation runs
- THEN the extent MUST remain inert

### Requirement: Extent receipts preserve producer and consumer boundaries

r[molten.world_extents.receipts] Extent receipts MUST separately record producer bundle evidence, consumer remeasurement, mapping observations, activation admission, unmap, and non-claims. They MUST NOT claim build correctness, semantic equivalence, sandboxing, authority, retention, or release eligibility.

#### Scenario: Mapping receipt claims sandboxing

- GIVEN a receipt treats W^X mapping as proof of host sandboxing
- WHEN receipt validation runs
- THEN Molten MUST reject the overclaim

### Requirement: Extent verification covers hostile layout and authority cases

r[molten.world_extents.verification] Molten MUST test exact mappings, identity separation, misalignment, overlap, truncation, substitution, target and ABI mismatch, partial closure, W^X denial, missing authority, explicit fallback, and overclaims.

#### Scenario: Focused extent rail runs

- GIVEN positive and negative fixtures use reviewed producer and mechanism revisions
- WHEN extent verification runs
- THEN it MUST report every supported profile and bounded non-claim
