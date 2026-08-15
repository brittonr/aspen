# Operator Workflow Delta: Dogfood Integration Boundary

### Requirement: Operator dogfood and production workflows are integration shells
r[molten.operator_workflow.modularity.integration_boundary] Operator dogfood, production soak, and NixOS VM workflows SHOULD be owned by integration-shell modules that consume stable runtime APIs and emit canonical review evidence.

#### Scenario: Dogfood workflow consumes stable runtime API
- GIVEN a dogfood workflow exercises node, retention, job, or transport behavior
- WHEN reviewers inspect the implementation
- THEN the workflow calls stable runtime or CLI-adapter APIs and packages resulting evidence rather than being imported by runtime cores

### Requirement: Runtime cores do not depend on operator integration modules
r[molten.operator_workflow.modularity.dependency_direction] Runtime, node, storage, transport, and policy cores MUST NOT import operator dogfood, production soak, or NixOS VM modules.

#### Scenario: Runtime-to-dogfood import is blocked
- GIVEN a runtime core imports an operator dogfood, prod-soak, or NixOS VM module
- WHEN dependency-boundary validation runs
- THEN validation fails or records the violation before release evidence is promoted

### Requirement: Operator integration receipts remain evidence only
r[molten.operator_workflow.modularity.evidence_only] Dogfood, production soak, and NixOS VM receipts MUST remain release-review or diagnostic evidence only and MUST NOT grant authority, policy, resource, provenance, retention, execution, transport, or source-gate trust by themselves.

#### Scenario: Complete dogfood evidence is reviewable
- GIVEN a dogfood workflow has complete child receipts and stable refs
- WHEN the operator workflow packages the run
- THEN it emits review evidence binding child refs and caveats without granting runtime authority

#### Scenario: Diagnostic log alone is denied
- GIVEN a VM or soak run has terminal logs but lacks required canonical receipts
- WHEN release-readiness evidence is evaluated
- THEN diagnostic logs alone are insufficient and the evidence is denied or marked unavailable

### Requirement: Operator integration modularity has positive and negative tests
r[molten.operator_workflow.modularity.tests] Operator workflow boundary refactors SHOULD include positive evidence aggregation tests and negative tests for missing child evidence, stale refs, unavailable VM execution, diagnostic-only logs, or overbroad release claims.

#### Scenario: Overbroad production claim is rejected
- GIVEN a dogfood or soak receipt claims broad production readiness without required supporting evidence
- WHEN validation evaluates the receipt
- THEN the claim is rejected or caveated before promotion
