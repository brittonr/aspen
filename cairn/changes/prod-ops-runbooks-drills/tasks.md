## Phase 1: Production profile and runbooks

- [ ] [serial] r[molten.prod_ops.deployment_profile] Define a production node deployment profile with explicit state layout, required adapters, source-gate inputs, resource limits, redaction settings, and live transport settings.
- [ ] [parallel] r[molten.prod_ops.operator_runbooks] Write operator runbooks for init, run, status, stop, evidence export, source-gate refresh, backup, restore, upgrade, rollback, and emergency stop.

## Phase 2: Recovery drills

- [ ] [serial] r[molten.prod_ops.state_backup_restore] Implement backup and restore drill receipts that verify ledgers, Redb stores, chunks, identity metadata, retention pins, and source-gate refs before restored operation.
- [ ] [serial] r[molten.prod_ops.upgrade_rollback_drill] Implement upgrade and rollback drills that bind migration receipts, smoke/dogfood evidence, irreversible-operation exclusions, and rollback eligibility.

## Phase 3: Observability

- [ ] [parallel] r[molten.prod_ops.observability_slo] Add structured observability and SLO receipts for adapter health, queue depth, control-loop liveness, resource pressure, retention drift, source-gate freshness, and live transport delivery.
- [ ] [parallel] r[molten.prod_ops.operator_runbooks] Add pass and denial tests for runbook command examples or fixtures where automation exists.
