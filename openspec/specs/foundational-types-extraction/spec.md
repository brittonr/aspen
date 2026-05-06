# foundational-types-extraction Specification

## Purpose
TBD - created by archiving change review-foundational-types-public-api. Update Purpose after archive.
## Requirements
### Requirement: Public API classification is explicit
The foundational extraction review MUST classify every reviewed crate as reusable API, compatibility shell, or internal-only helper before readiness labels change.
ID: foundational-types-extraction.classification-records-reusable-surface

#### Scenario: Public API classification is explicit evidence
ID: foundational-types-extraction.classification-records-reusable-surface.evidence
- GIVEN the review inventory includes `aspen-storage-types`, `aspen-traits`, `aspen-cluster-types`, `aspen-hlc`, `aspen-time`, and `aspen-constants`
- WHEN the reviewer records the family outcome
- THEN the result SHALL name canonical imports, compatibility shims, owner, readiness state, and any rejected public API surfaces.

#### Scenario: Reviewed family is extraction-ready in workspace
ID: foundational-types-extraction.classification-records-reusable-surface.ready
- GIVEN `docs/crate-extraction/foundational-types.md`, `docs/crate-extraction.md`, and `docs/crate-extraction/policy.ncl` have been updated
- WHEN the foundational family is marked `extraction-ready-in-workspace`
- THEN the docs SHALL record reusable API classifications for `aspen-constants`, `aspen-hlc`, `aspen-storage-types`, `aspen-cluster-types`, `aspen-traits`, and `aspen-time`
- AND the docs SHALL keep publishable/repo-split readiness blocked on license/publication policy.

### Requirement: Live boundary checks gate readiness
The foundational extraction review MUST use live dependency evidence instead of stale memory about resolved Redb table seams.
ID: foundational-types-extraction.live-boundary-evidence

#### Scenario: Live boundary checks gate readiness evidence
ID: foundational-types-extraction.live-boundary-evidence.evidence
- GIVEN the family is proposed for `extraction-ready-in-workspace` or a documented no-raise decision
- WHEN the evidence bundle is captured
- THEN it SHALL include the aspen-core no-default boundary checker, downstream fixture metadata, and negative dependency checks for Redb/runtime shells.

#### Scenario: Downstream fixture proves portable imports
ID: foundational-types-extraction.live-boundary-evidence.downstream-fixture
- GIVEN an independent downstream fixture depends on the foundational crates with `default-features = false`
- WHEN the fixture test and dependency tree run
- THEN it SHALL compile without depending on root `aspen`
- AND the normal dependency graph SHALL exclude Redb, Iroh, Axum, Hyper, Tokio, and Snix runtime shells.
