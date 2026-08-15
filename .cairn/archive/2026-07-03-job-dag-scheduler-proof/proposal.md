## Why

The job DAG scheduler is a state machine over node readiness, dependency completion, worker schedule state, and receipts. It needs proof that topological order is deterministic, cyclic or unknown-edge graphs deny, and no node runs before dependencies are satisfied.

## What Changes

- Add proof requirements for deterministic topological planning and scheduler readiness.
- Require generated DAG coverage for acyclic graphs, cycles, duplicate nodes, unknown edges, and unsatisfied dependency attempts.
- Require worker schedule receipts to bind completed indices, node refs, output refs, and replay identity.

## Impact

- **Files**: job DAG planning, worker scheduling, admission receipts, and tests.
- **Testing**: positive generated DAGs, negative malformed/cyclic graphs, no-unsatisfied-run checks, and schedule replay determinism.
