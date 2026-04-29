## Context

Aspen already has `scripts/openspec-preflight.sh`, but review history shows evidence coverage failures still recur. The gap is stricter validation and fixtures proving failure modes.

## Goals / Non-Goals

**Goals:** make missing or placeholder task evidence fail deterministically; keep failure output short and actionable.
**Non-Goals:** replace human review, run full builds automatically, or change OpenSpec archive semantics.

## Decisions

### 1. Extend preflight instead of adding a new tool

**Choice:** Strengthen `scripts/openspec-preflight.sh` and add fixture tests.
**Rationale:** This is already the required completion guardrail, so stronger behavior reaches all changes.
**Implementation:** Parse checked tasks, verification coverage entries, evidence paths, git tracking state, and file content length/placeholder markers.

### 2. Treat placeholder evidence as failure

**Choice:** Reject empty files and known placeholder phrases such as `pending`, `TODO`, or `placeholder` in evidence artifacts that are cited for checked tasks.
**Rationale:** Several failures come from files existing without proof.
**Implementation:** Keep a small explicit denylist and allow an override only for archived historical evidence not cited by a checked task.

## Risks / Trade-offs

**False positives on intentional tiny evidence** → Require non-empty content and task coverage; do not require large transcripts.

## Validation Plan

Add positive and negative fixtures, run shell tests, then run the updated preflight against at least one active or archived Aspen change.
