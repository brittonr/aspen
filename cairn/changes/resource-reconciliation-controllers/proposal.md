# Change: resource-reconciliation-controllers

## Why

Kubernetes controllers are powerful because each controller owns a small convergence loop: compare desired state to observed state, compute a plan, apply admitted effects, and update status. Molten needs the same convergence discipline, but with deterministic pure cores, capability-gated effects, and canonical receipts.

## What

- Define a controller reconciliation contract for resource-shaped desired/observed state.
- Require reconcilers to compute pure action plans before any adapter effect runs.
- Add deterministic work-queue, coalescing, retry, and backoff receipts.
- Bind apply results and status updates to the plan, generation, policy, authority, and effect evidence.

## Impact

Runtime services can converge declarative resources without bespoke polling loops or hidden side effects. Operators get receipts explaining why a controller did or did not act.
