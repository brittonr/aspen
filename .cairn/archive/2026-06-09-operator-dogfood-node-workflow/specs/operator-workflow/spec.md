# Operator Workflow Delta: Local Node Dogfood

### Requirement: Dogfood reports are canonical pass evidence
r[molten.operator_dogfood_node_workflow.spec.report] The local dogfood workflow MUST emit a canonical report whose decision is derived from step receipts, replay status, redaction checks, and gate receipts.

#### Scenario: Complete dogfood pass
- GIVEN a clean state root and admitted operator authority
- WHEN the local dogfood workflow completes all mandatory steps
- THEN it emits a `dogfood-report-v1` with decision `pass`
- AND the report binds startup, service, remote, job, catalog, repro, gate, and shutdown receipts

#### Scenario: Missing step receipt denies
- GIVEN a dogfood workflow where a mandatory step lacks a canonical receipt
- WHEN the final report is built
- THEN the report decision is `deny`
- AND no release gate receipt is emitted

### Requirement: Release gates exclude non-replayable evidence
r[molten.operator_dogfood_node_workflow.spec.release_gate] A dogfood release gate MUST require deterministic or recorded pass evidence for mandatory steps and MUST exclude unrecorded live diagnostics.

#### Scenario: Live diagnostic does not gate release
- GIVEN a dogfood report containing a live Iroh diagnostic step without recorded delivery/effect logs
- WHEN a release gate is requested
- THEN the gate denies or excludes that step from pass evidence according to policy

### Requirement: Operator bypasses are explicit
r[molten.operator_dogfood_node_workflow.spec.no_hidden_bypass] Operator workflows MUST NOT use hidden runtime backdoors; privileged actions MUST be represented as explicit capability-bearing requests with receipts.

#### Scenario: Unauthorized operator action denied
- GIVEN an operator workflow step without required authority
- WHEN the step attempts to install or execute an artifact
- THEN Molten emits a denial receipt
- AND the dogfood report records the failed step
