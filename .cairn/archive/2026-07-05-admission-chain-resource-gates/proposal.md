# Change: admission-chain-resource-gates

## Why

Kubernetes' admission chain is worth adapting because it gives every resource change a mandatory validation and defaulting boundary before persistence. Molten already has stronger policy materials than Kubernetes admission, but resource-shaped changes need one ordered, receipt-bearing chain so defaulting, mutation, validation, authority, and evidence gates cannot drift apart.

## What

- Add an ordered resource-admission chain for canonical resource create, update, status, delete, and reconcile-apply intents.
- Require deterministic defaulting and reviewed mutation rules to emit evidence before they alter a candidate resource.
- Isolate status updates from desired-state mutations.
- Add fail-closed diagnostics for malformed resources, unauthorized mutation, stale generation, missing policy/evidence refs, and status writes that attempt desired-state changes.

## Impact

Every resource lifecycle change becomes auditable before commit. Controllers and operators can rely on one admission receipt shape instead of bespoke checks, while Molten remains capability/evidence-first instead of adopting Kubernetes webhooks or RBAC semantics.
