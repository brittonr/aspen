# Node Runtime Specification

## Purpose

Defines the `node-runtime` capability.

## Requirements

### Requirement: Production deployment profile
r[molten.prod_ops.deployment_profile] Molten MUST define an explicit production node deployment profile that records required adapters, state-root layout, source-gate evidence refs, resource limits, redaction/logging settings, live transport settings, and startup/shutdown expectations as canonical evidence.

#### Scenario: Node starts with production profile evidence
- GIVEN an operator starts a node under the production deployment profile
- WHEN startup receipts are emitted
- THEN the receipts bind the profile ref, required adapter refs, resource limits, source-gate refs, and redaction settings
- AND startup denies if required profile evidence is missing or stale.

### Requirement: State backup and restore drill evidence
r[molten.prod_ops.state_backup_restore] Molten MUST provide backup and restore drill evidence for local ledgers, Redb stores, chunk-store state, retention pins, node identity metadata, and source-gate refs, and MUST verify restored refs before normal control operations resume.

#### Scenario: Tampered backup denies restore
- GIVEN a backup bundle with a missing or tampered ledger, chunk, Redb index, retention, or source-gate member
- WHEN a restore drill verifies the bundle
- THEN Molten emits a deny receipt and MUST NOT resume normal production control operations from that restored state.

### Requirement: Production observability and SLO evidence
r[molten.prod_ops.observability_slo] Molten MUST emit structured observability evidence for node health, adapter health, queue depth, control-loop liveness, resource pressure, source-gate freshness, retention drift, receipt import/export failures, and live transport delivery health.

#### Scenario: Observability snapshot reports degraded resource pressure
- GIVEN a running production-profile node with queue or resource pressure over its configured threshold
- WHEN an observability snapshot is emitted
- THEN the snapshot records the degraded status, relevant resource refs, and operator diagnostics without treating logs as canonical pass evidence.

### Requirement: Upgrade and rollback drills
r[molten.prod_ops.upgrade_rollback_drill] Molten MUST support upgrade and rollback drills that bind migration receipts, copied-state smoke or dogfood evidence, rollback eligibility, irreversible-operation exclusions, and post-rollback verification receipts.

#### Scenario: Irreversible migration blocks rollback claim
- GIVEN an upgrade plan includes an irreversible migration or destructive retention action without explicit rollback exclusion evidence
- WHEN rollback eligibility is evaluated
- THEN the rollback drill emits a deny receipt rather than claiming safe rollback.

### Requirement: Operator runbooks are evidence-backed
r[molten.prod_ops.operator_runbooks] Molten SHOULD provide operator runbooks for init, run, status, stop, evidence export, source-gate refresh, backup, restore, upgrade, rollback, and emergency stop, and MUST distinguish canonical receipts from auxiliary logs or summaries.

#### Scenario: Runbook points to canonical artifacts
- GIVEN an operator follows a production runbook
- WHEN the runbook references a successful operation
- THEN it names the canonical receipt, evidence bundle, or verification artifact required for review instead of relying on terminal output alone.
