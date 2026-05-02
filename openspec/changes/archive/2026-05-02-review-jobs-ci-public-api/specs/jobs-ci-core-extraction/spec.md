## MODIFIED Requirements

### Requirement: Reviewable evidence [r[jobs-ci-core-extraction.reviewable-evidence]]

The jobs/CI core extraction SHALL produce durable evidence linked from task and verification artifacts, including owner/public API review evidence before any readiness raise.

#### Scenario: Evidence linked from verification [r[jobs-ci-core-extraction.reviewable-evidence.evidence-linked-from-verification]]

- GIVEN implementation or review tasks are marked complete

- WHEN the change is prepared for review

- THEN `verification.md` SHALL cite checked-in evidence for inventory, dependency graphs, core tests, downstream fixture checks, negative boundary checks, readiness checker output, compatibility checks, owner/public API review scope, and OpenSpec validation

## ADDED Requirements

### Requirement: Owner/public API review [r[jobs-ci-core-extraction.owner-public-api-review]]

The jobs/CI core extraction SHALL require owner/public API review before raising readiness above `workspace-internal`.

#### Scenario: Review records canonical reusable surfaces [r[jobs-ci-core-extraction.owner-public-api-review.canonical-surfaces-recorded]]

- GIVEN `aspen-jobs-core`, `aspen-ci-core`, and `aspen-jobs-protocol` expose reusable jobs/CI contracts

- WHEN the owner/public API review is started or completed

- THEN the manifest SHALL record canonical reusable public API surfaces, compatibility re-export owners, runtime-shell exclusions, and retention or removal criteria

#### Scenario: Readiness raise uses fresh evidence [r[jobs-ci-core-extraction.owner-public-api-review.fresh-evidence-required]]

- GIVEN the jobs/CI family is proposed for `extraction-ready-in-workspace`

- WHEN the readiness change is prepared

- THEN fresh downstream metadata, negative boundary evidence, compatibility evidence, readiness checker output, and verification task coverage SHALL be checked in under the active change

- AND publishable or repo-split states SHALL remain blocked pending human license/publication policy
