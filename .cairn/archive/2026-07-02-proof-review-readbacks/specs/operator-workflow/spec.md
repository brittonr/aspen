## ADDED Requirements

### Requirement: Requirement-centered proof readback
r[molten.operator.proof_readback.requirement_view] Molten SHOULD provide a deterministic proof readback grouped by requirement id for local and release review.

#### Scenario: Readback names requirement evidence
- GIVEN a traceability manifest with covered requirements
- WHEN proof readback is rendered
- THEN each requirement section names its positive and negative evidence refs.

### Requirement: Readback shows evidence chain
r[molten.operator.proof_readback.evidence_chain] Proof readbacks SHOULD show verification-run receipts, aggregate proof manifests, child obligation refs, artifact refs, and gate receipts that explain how coverage was satisfied.

#### Scenario: Aggregate proof expands to children
- GIVEN a requirement covered by an aggregate proof manifest
- WHEN readback renders the requirement
- THEN it lists the child obligation refs that satisfy the requirement.

### Requirement: Readback includes scope caveats
r[molten.operator.proof_readback.scope_caveats] Proof readbacks MUST include explicit caveats that summaries are non-normative and do not grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation trust, or permission to bypass subsystem gates.

#### Scenario: Readback cannot override deny receipt
- GIVEN a canonical gate receipt with decision `deny`
- WHEN a readback is rendered
- THEN the readback cannot present that evidence as pass and must identify the deny decision.

### Requirement: Readback renders actionable gaps
r[molten.operator.proof_readback.gap_diagnostics] Proof readbacks SHOULD group missing-positive, missing-negative, stale-reference, unsupported, and exempt entries with actionable next evidence requirements.

#### Scenario: Missing negative is visible
- GIVEN a requirement missing negative coverage
- WHEN readback is rendered
- THEN the requirement appears in a missing-negative group with the required evidence kind.

### Requirement: Proof readback CLI
r[molten.operator.proof_readback.cli] Molten SHOULD expose a CLI command or release-review surface that renders proof readbacks from traceability manifests and proof receipts.

#### Scenario: Operator renders release proof readback
- GIVEN a release traceability manifest and proof receipt set
- WHEN the operator invokes the readback command
- THEN Molten renders a compact deterministic summary and can write a canonical readback artifact.

### Requirement: Readback Hegel properties
r[molten.operator.proof_readback.hegel_properties] Proof readback rendering SHOULD include Hegel RS property tests for stable ordering, duplicate suppression, gap grouping, summary-count consistency, and non-normative caveat preservation.

#### Scenario: Generated readback remains sorted
- GIVEN Hegel RS generates an unordered set of requirement entries
- WHEN readback rendering runs
- THEN the rendered requirement groups are deterministic and summary counts match the canonical entries.

### Requirement: Proof readback documentation
r[molten.operator.proof_readback.docs] Operator documentation SHOULD explain how to inspect proof readbacks, follow evidence refs, identify gaps, and treat readbacks as rendered views over canonical receipts.

#### Scenario: Reviewer follows readback docs
- GIVEN a reviewer receives a release proof readback
- WHEN they follow the documentation
- THEN they can locate canonical receipts for positive, negative, stale, and exempt evidence.
