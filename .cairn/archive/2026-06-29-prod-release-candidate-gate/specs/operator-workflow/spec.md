## ADDED Requirements

### Requirement: Production release-candidate validation matrix
r[molten.prod_release_candidate.full_validation_matrix] Molten MUST define a production release-candidate validation matrix that binds the current Rust validation, hermetic nextest, Nix checks, Cairn strict validation, Octet strict source gate, dogfood-local-node check, release bundle verification, promotion summary, and export verification evidence for the same candidate.

#### Scenario: Candidate passes only with current full evidence
- GIVEN a candidate source tree and Nix input set
- WHEN the production release-candidate gate evaluates the candidate
- THEN it accepts only passing, current, mutually bound validation evidence for that candidate
- AND it emits deny diagnostics for missing, stale, failed, or mismatched evidence.

### Requirement: Current source-gate evidence is required
r[molten.prod_release_candidate.source_gate_current] Molten MUST require current Octet source-gate evidence for production release candidates and MUST distinguish source-remediated-zero from configuration-clean evidence that still depends on disabled lint-family caveats.

#### Scenario: Configuration-clean caveat limits promotion
- GIVEN a candidate whose strict Octet gate passes only under documented disabled lint-family caveats
- WHEN the production release-candidate gate is asked to approve broad production use
- THEN the gate denies broad promotion or records the caveat as a pilot-scope limiter rather than claiming source-remediated zero.

### Requirement: Release-candidate receipt binds promotion evidence
r[molten.prod_release_candidate.evidence_bundle_promotion] Molten MUST emit a canonical production release-candidate receipt that binds dogfood output refs, release evidence bundle verification refs, promotion gate refs, signed promotion or keyring verification refs where available, promotion summary refs, export verification refs, and source-gate refs.

#### Scenario: Stale release bundle denies candidate
- GIVEN a release evidence bundle whose verified members do not match the candidate dogfood output or source-gate refs
- WHEN the production release-candidate receipt is generated
- THEN it emits a deny decision with diagnostics before any production pilot decision can pass.

### Requirement: Production pilot decision is explicit and scoped
r[molten.prod_release_candidate.pilot_decision] Molten MUST record production-pilot decisions explicitly, including allowed workloads, denied workloads, rollback triggers, stop-the-line conditions, operator review refs, and evidence-only caveats.

#### Scenario: Candidate is accepted for limited pilot only
- GIVEN all required release-candidate evidence passes but known caveats remain for live distributed soak or source-remediated-zero completeness
- WHEN the pilot decision is recorded
- THEN the receipt may pass only for the named constrained pilot scope
- AND it MUST deny or exclude broad customer-critical or irreversible destructive workloads.
