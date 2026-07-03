# Design: external live pilot soak evidence

## Scope

This change defines the next constrained production-readiness slice after local dogfood and NixOS VM multi-node evidence. It targets operator-managed multi-host evidence sufficient for a limited internal pilot, while explicitly denying broad production claims.

## Proof checklist

- **Proof claim**: a named multi-host pilot workflow can run through live node-control, remote dataspace/service exchange, job execution, coordination, retention/readback review, replay verification, network diagnostics, resource envelope checks, and rollback/stop-the-line evidence under explicit scope limits.
- **Out of scope**: broad customer-critical production, fleet-scale guarantees, adversarial security proof, irreversible destructive operations, global WAN correctness, and treating transport observations as authority.
- **Trusted assumptions**: operators provision the hosts, credentials, network, and state roots according to the runbook; live timing observations are diagnostic unless recorded in canonical receipts.
- **Positive evidence**: a complete pilot soak receipt with child refs for node-control workflow, peer admission, authority grant, service exchange, job result, coordination report, retention readback or clearance, replay verify/index, diagnostics, resource envelope, and rollback drill.
- **Negative evidence**: missing peer admission, missing authority, stale ticket, failed replay, degraded diagnostics outside threshold, resource envelope breach, missing retention review, and over-broad pilot scope all deny pilot decision.
- **Canonical refs**: pilot run ref, node ids, peer ticket refs, authority grant refs, workflow bundle refs, job receipt refs, coordination refs, retention refs, replay refs, diagnostics refs, resource refs, rollback refs, and pilot decision ref.
- **Regeneration command**: documented operator runbook commands for the constrained external/live pilot, plus local fixture tests for deny paths.

## Functional core

Represent pilot decision as pure validation over child evidence refs, decisions, scope declarations, caveats, and thresholds. Shell code may orchestrate host commands and collect artifacts, but pilot pass/deny remains a deterministic decision over canonical evidence.

## Non-goals

- No automatic fleet deployment.
- No destructive retention execution by default.
- No production trust from Iroh transport identity, diagnostics, or logs alone.
- No bypass of authority, policy, provenance, resource, source-gate, retention, or replay gates.
