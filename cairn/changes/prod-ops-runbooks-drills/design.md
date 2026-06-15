## Context

The node runtime currently exposes explicit state roots, startup/shutdown receipts, adapter receipts, control-loop receipts, supervisor-policy receipts, and dogfood workflows. Production operation needs those pieces composed into repeatable drills with documented failure behavior.

## Design

### Deployment profile

Define a production node profile that names required adapters, explicit state-root layout, source-gate requirements, identity storage rules, retention policies, resource limits, logging/redaction settings, and live transport configuration. The profile should produce a receipt so operators can tell what was actually launched.

### Backup and restore

Backups should be content-addressed evidence bundles over:

- local evidence ledger artifacts;
- Redb metadata stores;
- chunk manifests/chunks and retention pins;
- node identity public metadata, never raw secrets;
- source-gate and release-candidate receipts needed to interpret the state.

Restore drills must verify refs before use, deny missing/tampered state, and start a node in recovery mode before allowing normal control operations.

### Observability and SLOs

Define structured health and observability receipts for adapter health, queue depth, resource budget pressure, source-gate freshness, receipt import/export failures, retention drift, live transport delivery, and control-loop liveness. Logs remain auxiliary; canonical receipts and metrics snapshots are the production evidence.

### Upgrade and rollback drills

Upgrade drills should run against a copied state root, verify migration receipts, run a smoke/dogfood subset, and record rollback eligibility. Rollback must prove that prior state refs and required receipts are available and that irreversible migrations or destructive retention work are excluded unless explicitly admitted.

### Non-goals

- Do not add hidden operator bypasses.
- Do not store private key material in backup evidence bundles.
- Do not treat observability logs as canonical pass evidence.
