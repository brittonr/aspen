# Molten World Commit Specification

## Purpose

Define one immutable Molten identity for a coherent, typed, recoverable runtime snapshot without transferring authority or evidence ownership.

## Requirements

### Requirement: Molten world commits have canonical product-owned identity

r[molten.world_commit.core] Molten MUST define `molten-world-commit-v1` as canonical Preserves data. It MUST derive identity with domain-separated BLAKE3 framing over the complete immutable core.

#### Scenario: Equivalent worlds have one identity

- GIVEN two callers supply semantically equal version, parent, profile, and typed-root values in different input orders
- WHEN Molten canonicalizes and identifies both commits
- THEN both results MUST have identical canonical bytes and commit identities

#### Scenario: Behavior-relevant input changes

- GIVEN one parent, root identity, root type, profile, or version differs
- WHEN Molten identifies the changed commit
- THEN the resulting commit identity MUST differ

### Requirement: Every root retains its semantic domain

r[molten.world_commit.typed_roots] Molten MUST use distinct root-reference types for artifacts, schemas, durable state, tasks, history, effects, scheduler state, time, entropy, runtime profiles, policies, authority observations, and optional opaque machine snapshots. It MUST reject cross-domain substitution.

#### Scenario: Complete logical profile is admitted

- GIVEN a logical profile supplies every required typed root within declared bounds
- WHEN world-commit validation runs
- THEN validation MUST admit the typed root set for capture planning

#### Scenario: Artifact identity is supplied as task state

- GIVEN a caller places a valid artifact digest in the task-root field
- WHEN world-commit validation runs
- THEN validation MUST reject the reference as a domain mismatch

### Requirement: Capture publishes only a coherent observed cut

r[molten.world_commit.capture] Molten MUST publish a world commit only after every required immutable root is durable and every mutable revision fence still matches its captured observation. Drift, missing roots, incomplete inventories, or uncertain publication MUST NOT produce a success receipt.

#### Scenario: Every observation remains current

- GIVEN all required roots are durable and every rechecked revision equals its captured revision
- WHEN final commit publication succeeds
- THEN Molten MUST report one published world-commit identity and the exact observed fences

#### Scenario: Scheduler revision changes during capture

- GIVEN the scheduler root changes after initial observation
- WHEN the shell rechecks revision fences
- THEN publication MUST stop and the prior store state MUST NOT contain a successful commit record for that capture

### Requirement: Restore planning remains separate from runtime admission

r[molten.world_commit.restore] Molten MUST validate root closure and produce a deterministic restore plan without performing I/O. The shell MUST rerun current schema, artifact, policy, authority, resource, runtime, and effect admission before execution.

#### Scenario: Complete commit produces a restore plan

- GIVEN every required typed object exists and passes identity and schema checks
- WHEN restore planning runs
- THEN it MUST return a deterministic ordered adapter plan

#### Scenario: Commit is intact but current authority is absent

- GIVEN closure validation passes but current runtime authority admission denies
- WHEN the shell attempts activation
- THEN execution MUST remain denied even though commit integrity passed

### Requirement: Signatures and evidence remain detached

r[molten.world_commit.detached_evidence] Molten MUST exclude signatures, attestations, mutable head claims, currentness observations, and operator annotations from world-commit hash inputs. Detached envelopes MAY bind those facts to the commit identity.

#### Scenario: New evidence arrives

- GIVEN a valid commit receives an additional detached evidence envelope
- WHEN the evidence is stored
- THEN the world-commit identity MUST remain unchanged

#### Scenario: Attestation appears inside core bytes

- GIVEN candidate core data contains an embedded signature or attestation field
- WHEN canonical validation runs
- THEN the candidate MUST be rejected as an unsupported core schema

### Requirement: Verification covers success and denial boundaries

r[molten.world_commit.verification] Molten MUST test canonical identity, typed roots, coherent capture, closure, restore planning, opaque-profile boundaries, malformed inputs, stale observations, missing objects, domain confusion, secret disclosure, and overclaims.

#### Scenario: Focused world-commit rail runs

- GIVEN positive and negative fixtures use the reviewed schema and dependency cohort
- WHEN the focused verification rail runs
- THEN it MUST report all supported root profiles and bounded non-claims

## Executable-extent requirements

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
