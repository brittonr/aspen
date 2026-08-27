# Molten World Commit Specification Delta

## Purpose

Define one immutable Molten identity for a coherent, typed, recoverable runtime snapshot without transferring authority or evidence ownership.

## ADDED Requirements

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
