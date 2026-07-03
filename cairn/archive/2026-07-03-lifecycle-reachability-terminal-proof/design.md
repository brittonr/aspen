# Design: lifecycle reachability and terminal proof

## Scope

This change proves the lifecycle graph topology: which states are reachable from `Declared`, which states lead to cleanup, and which states have no outgoing semantic lifecycle edges.

## Proof checklist

- **Proof claim**: the lifecycle graph reachable from `Declared` matches the specified graph, and `Cleaned` has no outgoing passing transition.
- **Out of scope**: whether a supervisor chooses restart or cleanup in a concrete operational incident.
- **Trusted assumptions**: graph reachability is computed from the same finite allowed-edge relation used by receipt evaluation.
- **Positive evidence**: valid paths from declared through ready, degraded, failed, restarting, stopped, and cleaned are reachable according to the relation.
- **Negative evidence**: shortcuts, cleaned-state exits, and stopped/failed/restarting edges outside the relation deny.
- **Canonical refs**: lifecycle receipts remain the canonical evidence for individual edges.
- **Regeneration command**: `cargo test lifecycle`.

## Graph proof shape

Tests should compute reachability over the pure edge table rather than hand-maintaining an independent duplicate graph. Negative tests should still name critical forbidden shortcuts explicitly so review failures are readable.

## Non-goals

- No new restart-budget semantics.
- No process-level liveness guarantee that cleanup eventually happens.
