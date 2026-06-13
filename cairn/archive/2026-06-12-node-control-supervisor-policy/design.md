# Design: Node Control Supervisor Policy

## Artifacts
- `node-control-supervisor-policy-v1` binds bounded restart count, restart window ticks, heartbeat timeout ticks, shutdown drain ticks, explicit stale-lock recovery mode, policy refs, evidence refs, and checks.
- `node-control-supervisor-receipt-v1` binds decision, supervisor operation, startup receipt, optional service lock, optional supervisor policy ref, topic, diagnostics, and checks.
- `node-control-service-run-receipt-v1` now records optional supervisor policy and supervisor receipt refs so the service run can be replayed against the policy evidence that governed it.

## Workflow
1. `molten node supervisor-policy-fixture` writes/imports deterministic policy artifacts for local operator workflows.
2. `molten node serve --supervisor-policy` imports the policy before service-run side effects.
3. Stale service locks deny by default. If the policy explicitly allows stale-lock recovery, the runner writes a `stale-lock-recover` supervisor receipt before replacing the lock.
4. Every policy-governed service start writes a restart-attempt supervisor receipt or denies before taking a service lock when the bounded restart count is exceeded.
5. Duplicate active runners write duplicate-runner denial receipts when policy evidence is present and otherwise still fail closed through the service-run receipt.
6. Shutdown handling writes a shutdown-drain supervisor receipt and denies the service run if the observed drain exceeds the policy bound.

## Gate ordering
Supervisor policy gates protect service-runner lifecycle decisions only. They do not satisfy node-control authority, peer bootstrap, resource policy, delivery idempotency, or payload provenance. Remote/live ingress still reaches side effects only through durable inbox and dispatch gates.
