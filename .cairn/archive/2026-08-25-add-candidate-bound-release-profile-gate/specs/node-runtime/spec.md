# Node Runtime Delta

## ADDED Requirements

### Requirement: Release profile validation binds the reviewed candidate

r[molten.prod_ops.release_profile.candidate_binding] Release-tier profile validation MUST require one valid, non-placeholder candidate content reference and MUST record that reference with the reviewed evidence references.

#### Scenario: Candidate-bound release profile passes

- GIVEN a release-tier profile with a valid candidate reference and complete non-placeholder evidence
- WHEN release profile validation runs
- THEN the canonical validation value records the candidate and the accepted evidence references.

#### Scenario: Missing candidate denies release tier

- GIVEN a release-tier profile without a candidate reference
- WHEN release profile validation runs
- THEN validation denies with a missing-candidate diagnostic.

#### Scenario: Placeholder candidate denies release tier

- GIVEN a release-tier profile with an all-zero or fixture candidate reference
- WHEN release profile validation runs
- THEN validation denies before the profile can support release review.
