# Project specification delta

## ADDED Requirements

### Requirement: Runtime-spine Tracey repairs bind direct evidence
r[molten.project.runtime_spine_tracey.direct_repairs] Molten MUST remove a runtime-spine requirement from inherited Tracey debt only when a specific production location and focused positive or negative test directly support the accepted behavior.

#### Scenario: Reviewed requirement has direct evidence
- GIVEN a runtime-spine requirement selected for repair
- WHEN the repair manifest is validated
- THEN its implementation and verification paths contain direct recognized markers for that requirement.

### Requirement: Runtime-spine repair manifests are exact
r[molten.project.runtime_spine_tracey.exact_manifest] Molten MUST record each reviewed runtime-spine repair in typed metadata with its requirement identifier, source area, implementation path, verification path, and evidence scope.

#### Scenario: Manifest entry is incomplete
- GIVEN a repair entry without an implementation path or verification path
- WHEN Nickel validates the manifest
- THEN validation fails before the entry can change the baseline.

### Requirement: Runtime-spine repairs fail on drift
r[molten.project.runtime_spine_tracey.growth_denial] Molten MUST fail repository validation when a reviewed repair remains in the inherited baseline, loses a declared marker, or changes generated evidence without metadata regeneration.

#### Scenario: Direct marker is removed
- GIVEN a reviewed repair whose implementation marker is deleted
- WHEN the repository Nix check runs
- THEN the check fails and the inherited baseline cannot silently shrink.

### Requirement: Runtime-spine repairs preserve non-claims
r[molten.project.runtime_spine_tracey.non_claims] Molten MUST state that a bounded direct-repair batch does not prove complete runtime-spine coverage, behavioral correctness, release readiness, or whole-system correctness.

#### Scenario: Operator reads passing repair evidence
- GIVEN all listed repair checks pass
- WHEN the operator reviews the result
- THEN unreviewed runtime-spine requirements remain visible as explicit inherited debt.
