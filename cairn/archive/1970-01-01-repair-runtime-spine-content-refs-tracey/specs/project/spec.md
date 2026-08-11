# Project Specification Delta

## ADDED Requirements

### Requirement: Direct canonical content-ref evidence repairs

r[molten.project.runtime_spine_content_refs_tracey.direct_repairs] The project MUST remove a canonical content-ref requirement from inherited debt only when current production logic and focused tests directly support its complete accepted wording.

#### Scenario: Direct evidence is required
- GIVEN a canonical content-ref candidate
- WHEN the repair batch classifies it as repaired
- THEN the manifest names one production path and one verification path
- AND both paths contain exact Tracey markers for that requirement

### Requirement: Exact candidate manifest

r[molten.project.runtime_spine_content_refs_tracey.exact_manifest] The project MUST record all twelve candidates as ten direct repairs and two explicit rejections in typed deterministic evidence.

#### Scenario: Candidate accounting is exact
- GIVEN the canonical candidate queue
- WHEN the evidence manifest is exported
- THEN every candidate appears exactly once
- AND accepted and rejected counts match the declared batch boundary

### Requirement: Fail-closed inherited debt growth denial

r[molten.project.runtime_spine_content_refs_tracey.growth_denial] The project MUST regenerate the exact inherited baseline and MUST fail validation when repaired entries remain or rejected entries disappear.

#### Scenario: Baseline state drifts
- GIVEN the typed batch manifest and inherited baseline
- WHEN a repaired identifier remains or a rejected identifier is absent
- THEN validation fails before the evidence batch is accepted

### Requirement: Canonical content-ref repair non-claims

r[molten.project.runtime_spine_content_refs_tracey.non_claims] The project MUST state that this evidence repair does not prove universal helper-only construction, removal of all ad hoc formatting, content-ref trust, complete runtime-spine coverage, release readiness, or whole-system correctness.

#### Scenario: Evidence is reviewed
- GIVEN a passing direct-evidence batch
- WHEN its scope is interpreted
- THEN only the ten named requirements leave inherited debt
- AND every listed non-claim remains outside the batch
