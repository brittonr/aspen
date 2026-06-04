## Context

Molten has many pass-evidence artifacts but no single operator workflow exercising the stack as a user would. Aspen-style operational lessons emphasize local dogfood, durable receipts, replayable diagnostics, and confidence before rollout.

## Goals

- Define `operator-workflow-v1`, `operator-step-v1`, `operator-checkpoint-v1`, `dogfood-report-v1`, and `release-gate-receipt-v1`.
- Implement a named `molten dogfood local-node` workflow.
- Steps: initialize node state, start node, install artifact, start service, publish remote dataspace assertion, run job DAG sync/admit/execute, query catalog/MCP, export sealed/redacted repro, gate report, shutdown node.
- Bind every step to canonical request/receipt refs and deterministic replay status.
- Store reports and checkpoints in ledger/catalog and expose a concise operator summary.
- Fail closed on missing receipts, non-replayable production evidence, redaction leaks, stale policy/capability refs, or dirty state roots.

## Non-Goals

- No replacement for CI.
- No uncontrolled production deployment.
- No hidden operator bypass.
- No pass evidence from text logs alone.

## Records

```preserves
<operator-workflow-v1 "molten.operator.workflow.v1"
  <workflow-id "dogfood:local-node">
  <steps [<operator-step-v1 ...> ...]>
  <policy [<policy-ref> ...]>
  <capability [<authority-context-ref> ...]>
  <resource [<resource-ref> ...]>
  <replay-profile "deterministic"|"recorded"|"diagnostic">
  <checks [<check "no-hidden-bypass" "pass"> ...]>>
```

```preserves
<dogfood-report-v1 "molten.operator.dogfood-report.v1"
  <decision "pass"|"deny"|"diagnostic">
  <workflow <workflow-ref>>
  <checkpoints [<checkpoint-ref> ...]>
  <step-receipts [<step "start-node" <receipt-ref>> ...]>
  <gate-receipts [<gate-receipt-ref> ...]>
  <repro-bundles [<repro-bundle-ref> ...]>
  <diagnostics ["..." ...]>
  <checks [<check "deterministic-or-recorded" "pass"> ...]>>
```

## Workflow

The first workflow is local-only and intentionally small. It must run from a clean state root by default, but may import a fixture ledger. The final report is pass evidence only if all mandatory step receipts pass and all non-replayable steps are explicitly diagnostic and excluded from release gating.

## Release Gate

A release gate receipt binds dogfood report ref, node startup/shutdown refs, test harness gate refs, catalog query refs, repro bundle verify refs, and validation command refs. It is local developer/operator evidence, not a public attestation by itself.
