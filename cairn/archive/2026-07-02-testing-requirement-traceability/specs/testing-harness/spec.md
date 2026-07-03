## ADDED Requirements

### Requirement: Requirement coverage manifest
r[molten.testing.requirement_traceability.manifest] Molten MUST be able to generate a deterministic requirement coverage manifest that lists accepted and changed `r[...]` requirement ids, their source spec locations, positive verification evidence, negative verification evidence, validation commands, evidence artifact refs, and exemption status.

#### Scenario: Manifest records positive and negative coverage
- GIVEN accepted testing and evidence requirements with associated verification markers
- WHEN the requirement coverage manifest is generated
- THEN each covered requirement entry identifies its requirement id, source spec, positive test or evidence, negative test or evidence, validation command, and current coverage status.

#### Scenario: Documentation-only requirement is explicitly exempted
- GIVEN a requirement whose only required outcome is operator documentation
- WHEN the manifest is generated
- THEN the entry records a reviewed exemption class and supporting documentation evidence instead of silently appearing covered by unrelated tests.

### Requirement: Traceability gate requires covered evidence-bearing requirements
r[molten.testing.requirement_traceability.coverage_gate] Molten MUST provide a traceability gate that fails closed, or emits non-pass evidence, when an evidence-bearing or changed requirement lacks required positive and negative coverage and has no documented exemption.

#### Scenario: Missing negative coverage fails the gate
- GIVEN a changed evidence-bearing requirement with a positive test and no negative test or exemption
- WHEN the traceability gate runs
- THEN the gate fails closed with a diagnostic naming the requirement id and missing negative coverage.

#### Scenario: Complete coverage passes the gate
- GIVEN a changed evidence-bearing requirement with positive coverage, negative coverage, validation command evidence, and no stale refs
- WHEN the traceability gate runs
- THEN the gate emits pass evidence for that requirement coverage entry.

### Requirement: Traceability detects stale references
r[molten.testing.requirement_traceability.stale_detection] Traceability validation MUST detect stale requirement ids, missing test targets, missing validation commands, missing evidence artifacts, and references to deleted or renamed specs.

#### Scenario: Stale test reference fails closed
- GIVEN a manifest entry that points to a test target or fixture path that no longer exists
- WHEN traceability validation runs
- THEN validation fails closed with a stale-reference diagnostic.

#### Scenario: Removed requirement id is not counted as covered
- GIVEN a coverage entry for a requirement id that no longer appears in accepted specs or active change deltas
- WHEN traceability validation runs
- THEN the entry is reported as stale and cannot satisfy coverage for any current requirement.

### Requirement: Traceability fixtures cover success and failure
r[molten.testing.requirement_traceability.fixtures] Molten SHOULD test traceability validation with fixtures for complete coverage, missing positive coverage, missing negative coverage, stale requirement ids, missing test targets, missing evidence artifact refs, and documented exemptions.

#### Scenario: Missing evidence fixture is denied
- GIVEN a traceability fixture with a requirement entry whose evidence artifact ref is absent
- WHEN fixture validation runs
- THEN the validator reports a denial for the missing evidence artifact ref.

### Requirement: Traceability has an explicit gate surface
r[molten.testing.requirement_traceability.nix_surface] Molten SHOULD expose requirement traceability validation through an explicit Nix or Cairn command that can be invoked by release evidence review and local development.

#### Scenario: Release review invokes traceability gate
- GIVEN a release candidate source tree
- WHEN release evidence validation requests requirement traceability
- THEN the explicit gate command emits a machine-readable result and a compact summary without requiring manual source search.

### Requirement: Traceability summary is operator-readable
r[molten.testing.requirement_traceability.operator_summary] Molten SHOULD render a compact traceability summary grouped by covered, exempt, missing-positive, missing-negative, stale-reference, and unsupported requirement entries.

#### Scenario: Summary names actionable gaps
- GIVEN a manifest with missing negative coverage and stale references
- WHEN the operator summary is rendered
- THEN it names the affected requirement ids, gap class, and next validation evidence needed.

### Requirement: Traceability workflow is documented
r[molten.testing.requirement_traceability.docs] User-facing documentation SHOULD explain how to add positive coverage, negative coverage, validation commands, evidence refs, and exemptions when adding or changing requirements.

#### Scenario: Contributor updates coverage with a requirement
- GIVEN a contributor adding a new evidence-bearing requirement
- WHEN they follow the traceability documentation
- THEN they add both positive and negative coverage entries or a reviewed exemption before the traceability gate can pass.
