# Review foundational types public API Delta

## ADDED Requirements

### Requirement: Public API classification is explicit [r[foundational-types-extraction.classification-records-reusable-surface]]
The foundational extraction review MUST classify every reviewed crate as reusable API, compatibility shell, or internal-only helper before readiness labels change.

#### Scenario: Public API classification is explicit evidence [r[foundational-types-extraction.classification-records-reusable-surface.evidence]]
- GIVEN the review inventory includes `aspen-storage-types`, `aspen-traits`, `aspen-cluster-types`, `aspen-hlc`, `aspen-time`, and `aspen-constants`
- WHEN the reviewer records the family outcome
- THEN the result SHALL name canonical imports, compatibility shims, owner, readiness state, and any rejected public API surfaces.

### Requirement: Live boundary checks gate readiness [r[foundational-types-extraction.live-boundary-evidence]]
The foundational extraction review MUST use live dependency evidence instead of stale memory about resolved Redb table seams.

#### Scenario: Live boundary checks gate readiness evidence [r[foundational-types-extraction.live-boundary-evidence.evidence]]
- GIVEN the family is proposed for `extraction-ready-in-workspace` or a documented no-raise decision
- WHEN the evidence bundle is captured
- THEN it SHALL include the aspen-core no-default boundary checker, downstream fixture metadata, and negative dependency checks for Redb/runtime shells.
