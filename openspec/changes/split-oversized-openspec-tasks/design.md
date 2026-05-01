## Context

OpenSpec tasks are useful only when each can be implemented and verified with a bounded evidence set.

## Goals / Non-Goals

**Goals:** reduce ambiguous task completion; improve ordering.
**Non-Goals:** enforce exact duration estimates or block legitimate integration tasks.

## Decisions

### 1. Use requirement fan-out as an oversized-task signal

**Choice:** Flag implementation tasks that cover more than a configured number of requirement/scenario IDs.
**Rationale:** High fan-out often means many deliverables hidden in one checkbox.
**Implementation:** Parse `[covers=...]` tags and task prose for ID lists.

### 2. Permit explicit integration verification tasks

**Choice:** Verification tasks may cover many IDs when labeled as integration proof.
**Rationale:** One end-to-end check can validly prove multiple scenarios.
**Implementation:** Allow broad coverage under `V*` tasks with evidence expectations.

## Risks / Trade-offs

**Heuristic false positives** → Start as warnings except for missing dependency-order notes on explicitly delayed tasks.

## Verification Strategy

- `openspec-governance.task-size-and-ordering`: fixture suite exercises task fan-out, integration-proof exceptions, and dependency-order diagnostics.
- `openspec-governance.task-size-and-ordering.bounded-implementation-task-passes`: bounded implementation task fixture with small `[covers=...]` fan-out must pass.
- `openspec-governance.task-size-and-ordering.oversized-implementation-task-flagged`: oversized implementation task fixture must fail with split guidance.
- `openspec-governance.task-size-and-ordering.integration-verification-may-be-broad`: broad `V*` task labeled integration proof with evidence expectations must pass.
- `openspec-governance.task-size-and-ordering.dependency-order-explicit`: out-of-order later-task references and ambiguous foundation dependencies must fail unless a prerequisite note is explicit.

## Validation Plan

Fixtures for small tasks, broad integration task, oversized implementation task, and out-of-order prerequisite note.
