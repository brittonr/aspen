# Operator Workflow Delta

## ADDED Requirements

### Requirement: Release-readiness evidence binds one candidate source

r[molten.prod_release_candidate.evidence_source_binding] Molten MUST pair every release-readiness artifact reference with its candidate source reference and MUST require every bound source to equal the reviewed release candidate source.

#### Scenario: One candidate owns the full matrix

- GIVEN canonical evidence bindings for every required release-readiness category
- AND every binding identifies the reviewed candidate source
- WHEN the release-candidate gate evaluates the matrix
- THEN it emits a passing versioned receipt that preserves every artifact-to-source association.

#### Scenario: Mixed candidate evidence denies

- GIVEN one evidence binding identifies another candidate source
- WHEN the release-candidate gate evaluates the matrix
- THEN it denies the candidate before it emits a passing receipt
- AND it identifies the mismatched evidence category.

#### Scenario: Malformed or incomplete binding denies

- GIVEN an evidence binding has a malformed artifact reference, malformed source reference, or missing pair member
- WHEN the release-candidate gate evaluates the matrix
- THEN it denies the candidate with a bounded diagnostic.

### Requirement: Candidate binding has bounded meaning

r[molten.prod_release_candidate.evidence_binding_non_claim] Molten MUST state that candidate evidence binding validates declared identity associations only and MUST NOT claim that it proves external artifact truth or grants release authority.

#### Scenario: Bound evidence passes identity checks

- GIVEN all declared evidence bindings identify one candidate
- WHEN the gate emits a passing receipt
- THEN the receipt records the identity association
- AND operators still require the referenced validation, promotion, and pilot decisions for release review.
