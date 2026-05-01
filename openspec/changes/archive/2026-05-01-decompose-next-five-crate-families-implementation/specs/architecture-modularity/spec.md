## MODIFIED Requirements

### Requirement: Policy and checker cover the next wave [r[architecture.modularity.next-decomposition-policy-covers-wave]]
The crate-extraction policy and deterministic readiness checker SHALL cover every selected target family with explicit forbidden categories, allowed backend/domain exceptions, tested feature sets, representative consumers, downstream fixture metadata requirements, compatibility evidence requirements, and negative mutation cases.

#### Scenario: Policy contains selected family entries [r[architecture.modularity.next-decomposition-policy-covers-wave.policy-contains-selected-family-entries]]
- GIVEN `docs/crate-extraction/policy.ncl` is evaluated for the next decomposition wave
- WHEN the selected-wave policy entries are inspected
- THEN it SHALL define candidate-family coverage for `foundational-types`, `auth-ticket`, `jobs-ci-core`, `trust-crypto-secrets`, and `testing-harness`
- AND each selected family SHALL include owner, canonical class, allowed dependency categories or exceptions, forbidden dependency categories, tested feature sets, and representative consumers

#### Scenario: Checker validates selected family boundaries [r[architecture.modularity.next-decomposition-policy-covers-wave.checker-validates-selected-family-boundaries]]
- GIVEN `scripts/check-crate-extraction-readiness.rs --candidate-family <family>` is run for any selected family
- WHEN the family is one of `foundational-types`, `auth-ticket`, `jobs-ci-core`, `trust-crypto-secrets`, or `testing-harness`
- THEN the checker SHALL verify manifest completeness, readiness-state restrictions, direct dependency boundaries, transitive dependency boundaries, representative-consumer coverage, downstream fixture metadata evidence, and compatibility evidence
- AND negative mutations SHALL prove the checker fails on at least forbidden runtime dependency, missing owner, invalid readiness state, missing downstream fixture, and missing compatibility evidence cases for that family

### Requirement: Each selected family proves standalone usage and compatibility [r[architecture.modularity.next-decomposition-standalone-and-compatibility-proof]]
Every selected family SHALL include both positive and negative verification before readiness is raised: positive standalone compile/usage evidence with canonical APIs, negative boundary tests or checker evidence for app/runtime leaks, and compatibility evidence for Aspen consumers that still depend on the family through runtime features or existing public paths.

#### Scenario: Family evidence is stored under the implementation change [r[architecture.modularity.next-decomposition-standalone-and-compatibility-proof.family-evidence-stored-under-implementation-change]]
- GIVEN a selected family first-blocker slice is claimed complete
- WHEN evidence is saved for review
- THEN downstream fixture metadata, forbidden-dependency proof, compatibility transcripts, positive tests, and negative tests SHALL be stored under `openspec/changes/decompose-next-five-crate-families-implementation/evidence/`
- AND `openspec/changes/decompose-next-five-crate-families-implementation/verification.md` SHALL map the checked task text to those evidence paths
