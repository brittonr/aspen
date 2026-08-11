# Project specification delta

## ADDED Requirements

### Requirement: SAM service Tracey repairs bind direct evidence
r[molten.project.runtime_spine_sam_tracey.direct_repairs] Molten MUST remove a SAM service requirement from inherited runtime-spine debt only when a specific production location and focused positive or negative test directly support the complete accepted behavior.

#### Scenario: Reviewed requirement has direct evidence
- GIVEN a SAM service requirement selected for repair
- WHEN the batch manifest is validated
- THEN its implementation and verification paths contain direct recognized markers for that requirement.

### Requirement: SAM service repair manifests are exact
r[molten.project.runtime_spine_sam_tracey.exact_manifest] Molten MUST record each reviewed SAM service repair in typed metadata with its requirement identifier, source area, implementation path, verification path, and evidence scope.

#### Scenario: Manifest entry is incomplete
- GIVEN a repair entry without an implementation path or verification path
- WHEN Nickel validates the manifest
- THEN validation fails before the entry can change the baseline.

### Requirement: SAM service repairs fail on drift
r[molten.project.runtime_spine_sam_tracey.growth_denial] Molten MUST fail repository validation when a reviewed repair remains in the inherited baseline, loses a declared marker, duplicates an identifier, or changes generated evidence without metadata regeneration.

#### Scenario: Direct marker is removed
- GIVEN a reviewed repair whose implementation marker is deleted
- WHEN the repository Nix check runs
- THEN the check fails and the inherited baseline cannot silently shrink.

### Requirement: SAM service repairs preserve authority boundaries
r[molten.project.runtime_spine_sam_tracey.non_claims] Molten MUST preserve explicit service authority, logical-supervision, retention-policy, complete-coverage, release, and whole-system non-claims when this batch passes.

#### Scenario: Operator reads passing batch evidence
- GIVEN all thirteen listed repair checks pass
- WHEN the operator reviews the result
- THEN the result does not grant ambient authority, accept process parentage as supervision, bypass retention policy, or claim complete runtime-spine coverage.
