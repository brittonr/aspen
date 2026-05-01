# Design Template

## Context

Describe current behavior, constraints, and why this design is needed.

## Goals / Non-Goals

**Goals:** list the outcomes this change must achieve.
**Non-Goals:** list explicitly excluded work.

## Decisions

### 1. Decision Name

**Choice:** what was decided.
**Rationale:** why this option was chosen.
**Alternative:** what was rejected and why.
**Implementation:** how the choice will be implemented.

## Risks / Trade-offs

Document risks and mitigations.

## Verification Strategy

Required for changes that add, modify, or remove delta specs under `specs/`.
Map each changed requirement or scenario ID to concrete verification rails before tasks are approved.
Include both positive and negative/failure behavior where the delta spec defines rejection, failure, timeout, malformed, unauthorized, forbidden, invalid, or missing-input behavior.
If a rail cannot run in the current implementation slice, explicitly defer it with rationale and name the follow-up evidence or change.

Example:

- `domain.requirement-id`: positive check via `cargo test -p crate module::happy_path`; negative check via `cargo test -p crate module::rejects_invalid_input`.
- `domain.requirement-id.failure-scenario`: deferred until runtime fixture exists because external service credentials are unavailable; follow-up evidence will be saved under `<change>/evidence/runtime-negative.txt`.

## Validation Plan

List the smallest commands, fixtures, gates, or proofs that will demonstrate the verification strategy.
