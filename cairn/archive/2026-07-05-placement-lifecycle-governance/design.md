# Design: placement lifecycle governance

## Context

This change adapts Kubernetes scheduling and lifecycle ideas without adopting pods, nodes, kubelets, schedulers, or probe APIs. Molten placement applies to runtime services, actors, Wasm components, jobs, plugin hosts, workflow runners, and node-control operations.

## Placement model

The pure placement core should evaluate:

- resource requests, limits, and quota refs;
- priority class and preemption policy refs when admitted;
- placement constraints and affinity/anti-affinity summaries;
- taints and tolerations as explicit deny/preference inputs;
- node or worker capacity evidence refs;
- authority evidence for assigning work to a target.

The result is a placement decision, denial, or retry/defer plan with diagnostics.

## Lifecycle model

Lifecycle records should bind startup, liveness, readiness, graceful shutdown, restart, and terminal states. Probe evidence is input to status and restart decisions; it is not ambient authority. Restart/backoff decisions use named profiles and bind attempt counts, prior condition refs, and policy refs.

## GC and cleanup

GC is a controlled lifecycle operation. The shell may remove resources or artifacts only after the pure core proves owner/finalizer/pin/retention/authority gates are satisfied and emits a cleanup plan receipt.

## Boundaries

No runtime code should depend on Kubernetes probe endpoints, pod phases, CNI, CSI, service mesh, RBAC, or YAML. Molten uses its own Preserves records, capabilities, and evidence gates.
