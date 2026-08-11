# Project specification delta

## ADDED Requirements

### Requirement: Blob-ref job Tracey repairs bind direct evidence
r[molten.project.runtime_spine_job_dag_tracey.direct_repairs] Molten MUST remove a blob-ref job requirement from inherited runtime-spine debt only when a specific production location and focused positive or negative test directly support the complete accepted behavior.

#### Scenario: Reviewed requirement has direct evidence
- GIVEN a blob-ref job requirement selected for repair
- WHEN the batch manifest is validated
- THEN its implementation and verification paths contain direct recognized markers for that requirement.

### Requirement: Blob-ref job repair manifests are exact
r[molten.project.runtime_spine_job_dag_tracey.exact_manifest] Molten MUST record each reviewed blob-ref job repair in typed metadata with its requirement identifier, source area, implementation path, verification path, and evidence scope.

#### Scenario: Manifest entry is incomplete
- GIVEN a repair entry without an implementation path or verification path
- WHEN Nickel validates the manifest
- THEN validation fails before the entry can change the baseline.

### Requirement: Blob-ref job repairs fail on drift
r[molten.project.runtime_spine_job_dag_tracey.growth_denial] Molten MUST fail repository validation when a reviewed repair remains in the inherited baseline, loses a declared marker, duplicates an identifier, or changes generated evidence without metadata regeneration.

#### Scenario: Direct marker is removed
- GIVEN a reviewed repair whose implementation marker is deleted
- WHEN the repository Nix check runs
- THEN the check fails and the inherited baseline cannot silently shrink.

### Requirement: Blob-ref job repairs preserve non-claims
r[molten.project.runtime_spine_job_dag_tracey.non_claims] Molten MUST keep unsupported replay identity, full status coverage, and job-DAG integration requirements in inherited debt until direct evidence supports their complete wording.

#### Scenario: Operator reads passing batch evidence
- GIVEN all nine listed repair checks pass
- WHEN the operator reviews the result
- THEN unsupported blob-ref job requirements remain visible as explicit inherited debt.
