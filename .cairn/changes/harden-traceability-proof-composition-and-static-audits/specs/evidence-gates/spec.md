# Evidence gates deltas

## ADDED Requirements

### Requirement: Layered proof composition rejects denied children and accepts acyclic DAGs
r[molten.evidence_gates.layered_proof_sound_composition] `layered_proof_diagnostics` MUST reject any passing layer that binds a child layer whose decision is not `pass`, `layer_cycles` MUST distinguish true cycles from shared-descendant acyclic DAG diamonds using an active recursion stack, and `coverage_from_aggregate_proof` MUST fail closed when `manifest.decision` is not `pass`.

#### Scenario: Passing parent binding denied child is rejected
- GIVEN a release or gate layer with `decision = "pass"` referencing a pure-core child layer with `decision = "deny"`
- WHEN `layered_proof_diagnostics` evaluates the graph
- THEN it MUST emit a denial diagnostic and MUST NOT pass the layered manifest.

#### Scenario: Acyclic diamond DAG passes while cycle denies
- GIVEN a layered proof graph where two branches share one passing child layer with no cycle
- WHEN `layer_cycles` inspects the graph
- THEN it MUST accept the diamond DAG and MUST reject graphs containing a true back-edge cycle.

#### Scenario: Denied aggregate manifest yields no coverage
- GIVEN an aggregate proof manifest with `decision = "deny"`
- WHEN `coverage_from_aggregate_proof` is invoked
- THEN it MUST return a typed denial error rather than extracting coverage tuples.

### Requirement: Static authority audit hashes rule YAML bytes and scanned source digests
r[molten.evidence_gates.static_audit_content_bound_hashes] `ast_grep_runtime_authority_core` MUST derive `rule_bundle_hash` from actual YAML rule file bytes and `scan_scope_hash` from actual scanned file digests, MUST declare all active blocking rules and `node-state-adapters` in its default profile, and MUST derive `passed` from diagnostics rather than hardcoding `true`.

#### Scenario: Editing a YAML rule changes rule_bundle_hash
- GIVEN an ast-grep YAML rule pattern is modified without changing summary metadata
- WHEN `rule_bundle_hash` is computed
- THEN the resulting hash MUST change and invalidate stale scan receipts.

#### Scenario: Active node-state blocking rule is recognized by default profile
- GIVEN a finding emitted by `node-state-root-reacquisition.yml`
- WHEN the default Rust audit profile validates the scan input
- THEN the profile MUST recognize the rule as blocking and fail the receipt.
