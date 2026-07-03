# Design: job DAG scheduler proof

## Scope

This change proves the job DAG planning and worker scheduler state machine. It covers node/edge parsing, Trellis topological mapping, dependency readiness, worker request identity, stage receipts, output refs, and schedule replay.

## Proof checklist

- **Proof claim**: valid acyclic DAGs produce deterministic topological schedules, and a node is admitted to run only when all dependency indices are complete; malformed or cyclic DAGs deny before execution.
- **Out of scope**: correctness of the executable payload, remote worker trust, and external blob transport reliability.
- **Trusted assumptions**: node ids, content refs, and output refs are canonical where validators accept them.
- **Positive evidence**: generated acyclic DAGs produce stable order ids, dependency indices, stage receipts, and output-root selections.
- **Negative evidence**: duplicate nodes, unknown edge endpoints, cycles, unsatisfied dependencies, missing executable refs, and replay identity drift deny.
- **Canonical refs**: proof traces bind DAG refs, node refs, dependency indices, completed indices, worker request refs, stage receipt refs, and output refs.
- **Regeneration command**: `cargo test job`.

## Functional core

Topological planning, dependency readiness, and schedule-state transitions should remain pure over DAG and completed-index inputs. Worker shells execute only after admission receipts pass.

## Non-goals

- No proof that arbitrary job code is deterministic.
- No claim that remote workers faithfully execute without separate provenance and effect evidence.
