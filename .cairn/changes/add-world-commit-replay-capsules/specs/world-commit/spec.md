# Molten World Commit Specification Delta

## Purpose

Bind deterministic execution to exact world-commit transitions and package complete bounded closures for portable, authority-neutral replay.

## ADDED Requirements

### Requirement: Replay traces bind every expected world transition

r[molten.world_replay.transition_chain] Molten MUST define a canonical bounded transition trace that binds one initial world commit and an ordered sequence of transition inputs, deterministic profiles, expected parents, and expected successor commits.

#### Scenario: Every replayed step matches

- GIVEN a complete trace and closure for a supported logical profile
- WHEN Molten restores and replays each step
- THEN every actual successor commit MUST equal the expected successor before the next step runs

#### Scenario: One intermediate successor differs

- GIVEN a replay produces an unexpected commit before a later step
- WHEN transition verification compares the result
- THEN it MUST stop at the first mismatching step
- AND it MUST NOT report final-trace success

### Requirement: Replay divergence is complete-world and refs-only

r[molten.world_replay.divergence] Molten MUST compare expected and actual commit identities and typed roots. It MUST report the earliest differing step, root domain, and bounded field path without exposing secret bytes or bearer material.

#### Scenario: Entropy root diverges

- GIVEN all earlier steps match and one step produces a different entropy root
- WHEN divergence classification runs
- THEN the report MUST name that step and the entropy-root domain
- AND later differences MUST NOT replace the earliest result

### Requirement: Replay capsules bind complete typed closure

r[molten.world_replay.capsule] Molten MUST define a canonical capsule manifest over every required trace, commit, typed root, artifact, schema, policy, runtime cohort, snapshot descriptor, transition input, and content manifest. Each member MUST bind its role, identity, codec, and byte length.

#### Scenario: Complete capsule is exported

- GIVEN every reachable member is available and within declared bounds
- WHEN capsule planning runs
- THEN the resulting manifest MUST enumerate the complete typed closure with one stable identity

#### Scenario: Required schema is absent

- GIVEN one world root references a schema that is not in the capsule closure
- WHEN capsule validation runs
- THEN validation MUST fail before replay or import publication

### Requirement: Import validates before availability

r[molten.world_replay.import] Molten MUST validate capsule identity, canonical encodings, member bounds, complete closure, object identities, supported profiles, and protection policy before imported objects become available for restore or replay.

#### Scenario: Imported member is tampered

- GIVEN transport returns bytes that do not match the declared member identity
- WHEN import verification runs
- THEN Molten MUST reject the member and MUST NOT publish capsule availability

#### Scenario: Valid import completes

- GIVEN every member verifies and the complete closure passes
- WHEN import publication commits
- THEN Molten MAY report capsule availability without moving a branch or activating a runtime

### Requirement: Replay execution remains profile-bound and authority-neutral

r[molten.world_replay.execution_boundary] Molten MUST use the declared logical or opaque restore profile and MUST rerun current authority, artifact, schema, resource, runtime, and effect admission before execution. Capsule possession MUST NOT grant those admissions.

#### Scenario: Capsule is complete but authority is absent

- GIVEN capsule validation passes and current execution authority denies
- WHEN replay activation is requested
- THEN replay MUST remain denied

#### Scenario: Opaque profile targets a different cohort

- GIVEN an exact machine snapshot descriptor does not match the destination cohort
- WHEN replay planning runs
- THEN Molten MUST reject the profile without falling back to logical restore

### Requirement: Replay receipts preserve bounded claims

r[molten.world_replay.receipts] Replay and import receipts MUST bind trace, capsule, profile, horizon, closure, actual transitions, divergence, redaction, and dependency identities. They MUST NOT claim universal determinism, semantic equivalence, capability transfer, effect completion, or release eligibility.

#### Scenario: Focused replay rail runs

- GIVEN positive and negative fixtures use reviewed dependency cohorts
- WHEN the world replay verification rail runs
- THEN it MUST report exact supported profiles and all bounded non-claims

### Requirement: Replay verification covers success and denial paths

r[molten.world_replay.verification] Molten MUST test complete replay, stable identities, capsule round trips, first divergence, closure failures, malformed encodings, unsupported profiles, secret disclosure, missing authority, and import-overclaim cases.

#### Scenario: Negative corpus is incomplete

- GIVEN replay fixtures omit malformed, missing-closure, profile-mismatch, or authority-denial cases
- WHEN verification coverage is evaluated
- THEN the replay change MUST remain incomplete
