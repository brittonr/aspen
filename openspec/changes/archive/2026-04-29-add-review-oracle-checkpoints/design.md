## Context

Some review issues cannot be deterministically resolved from available artifacts. Today these become repeated human-route findings.

## Goals / Non-Goals

**Goals:** make ambiguity explicit and durable.
**Non-Goals:** require user approval for every routine ambiguity or block deterministic fixes.

## Decisions

### 1. Checkpoint record lives with the change

**Choice:** Store checkpoint notes under the change evidence directory or `verification.md`.
**Rationale:** Reviewers need repo-local context.
**Implementation:** Template fields: question, evidence, decision, owner, next action.

### 2. Done-review can recommend but not fake resolution

**Choice:** If evidence is insufficient, done-review emits an oracle-needed finding unless a checkpoint exists.
**Rationale:** Prevents overclaiming from incomplete evidence.
**Implementation:** Match repeated human-route classes from review metrics and require checkpoint artifacts.

## Risks / Trade-offs

**Process overhead** → Trigger only for repeated human-route classes or explicit ambiguous evidence gaps.

## Validation Plan

Fixture with checkpoint accepted and fixture without checkpoint flagged.
