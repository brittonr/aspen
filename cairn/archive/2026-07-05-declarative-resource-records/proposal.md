# Change: declarative-resource-records

## Why

Molten has envelopes, receipts, actors, workflows, jobs, plugins, and node-control records, but no shared declarative resource shape for things that should be reconciled. Kubernetes' most durable idea is not YAML or pods; it is the split between desired state, observed state, metadata, status, and lifecycle ownership. Molten needs that pattern in canonical Preserves terms so controllers, admission gates, GC, and operator UX do not invent incompatible record shapes.

## What

- Add a canonical Molten resource record model with typed identity, scope, metadata, desired-state ref, observed-state ref, generation, and evidence refs.
- Add status conditions with observed generation, reason, message, severity, evidence refs, and deterministic transition rules.
- Add owner refs and finalizers as evidence-bound lifecycle metadata that can block deletion until cleanup receipts exist.
- State explicitly that this borrows the declarative resource pattern from Kubernetes without claiming Kubernetes API, YAML, CRD, or controller-runtime compatibility.

## Impact

Future controllers and operators get a common resource vocabulary. Admission and reconciliation can gate the same canonical resource shape, status updates become reviewable evidence instead of log prose, and deletion/GC safety becomes explicit before effects run.
