## ADDED Requirements

### Requirement: External live pilot scope is explicit
r[molten.external_live_pilot_soak.scope_model] Molten MUST model external live pilot scope with named hosts or nodes, allowed workloads, denied workloads, rollback triggers, stop-the-line conditions, operator review refs, and evidence-only caveats.

#### Scenario: Over-broad pilot scope denies
- GIVEN pilot evidence that only covers a constrained internal workload
- WHEN the pilot decision requests broad production or irreversible destructive workload approval
- THEN the pilot decision denies or excludes the over-broad scope
- AND diagnostics identify the missing evidence classes.

### Requirement: External pilot operator runbook is reproducible
r[molten.external_live_pilot_soak.operator_runbook] Molten SHOULD document operator-runbook steps for multi-host setup, state roots, live tickets, authority grants, node-control workflow, artifact collection, replay readback, rollback, and teardown.

#### Scenario: Operator can rerun pilot collection
- GIVEN an operator follows the external pilot runbook
- WHEN the pilot workflow completes or denies
- THEN the runbook identifies the canonical artifacts to collect for review
- AND diagnostic logs remain secondary to receipts.

### Requirement: External pilot evidence bundle binds child workflows
r[molten.external_live_pilot_soak.evidence_bundle] External pilot evidence MUST bind child refs for node-control live workflow, peer admission, authority grant, remote dataspace or service exchange, blob-ref job execution, coordination apply, replay verification, network diagnostics, resource envelope, and rollback or stop-the-line evidence.

#### Scenario: Complete pilot bundle passes scope checks
- GIVEN all required child workflow receipts pass for the named pilot scope
- WHEN the pilot evidence bundle is validated
- THEN the bundle decision may pass for that constrained scope
- AND the bundle remains review evidence only.

### Requirement: External pilot positive workflow is covered
r[molten.external_live_pilot_soak.positive_workflow] Molten SHOULD provide a complete positive pilot workflow fixture or operator-managed evidence run that binds node-control, service exchange, blob-ref job, coordination, retention/readback, replay, diagnostics, resource, and rollback child evidence.

#### Scenario: Positive workflow binds required children
- GIVEN the positive pilot workflow evidence set
- WHEN pilot validation inspects required child refs
- THEN each required child class is present and scoped to the pilot workload.

### Requirement: External pilot negative denials are covered
r[molten.external_live_pilot_soak.negative_denials] Molten SHOULD test or record denial evidence for missing peer admission, missing authority, stale ticket, failed replay, diagnostics outside threshold, resource breach, missing retention review, and over-broad pilot scope.

#### Scenario: Missing peer admission denies
- GIVEN pilot evidence without a current peer admission receipt
- WHEN the pilot decision validator runs
- THEN the decision is `deny`
- AND diagnostics identify missing peer admission evidence.

### Requirement: External pilot decision denies missing boundary evidence
r[molten.external_live_pilot_soak.decision_receipt] External pilot decisions MUST deny when peer admission, authority, policy, resource, provenance, source-gate, replay, retention review, diagnostics, rollback, or freshness evidence required by the pilot scope is missing, stale, failed, or mismatched.

#### Scenario: Missing authority denies pilot decision
- GIVEN a live workflow bundle with transport evidence and no matching authority grant
- WHEN the external pilot decision evaluates the bundle
- THEN the decision is `deny`
- AND diagnostics state that transport evidence does not grant authority.

### Requirement: External pilot readback preserves caveats
r[molten.external_live_pilot_soak.release_readback] Operator and release readback for external pilot evidence MUST render pilot caveats and MUST NOT present constrained pilot evidence as broad production readiness.

#### Scenario: Pilot summary cannot override caveats
- GIVEN a passing constrained pilot decision
- WHEN release readback renders the pilot summary
- THEN the summary names the allowed and denied scopes
- AND it states that subsystem gates remain independently required.
