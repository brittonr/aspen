## Context

Preflight already verifies tracked paths but not all freshness properties reviewers need.

## Goals / Non-Goals

**Goals:** ensure final evidence maps to current paths and current state.
**Non-Goals:** inspect every line in long command transcripts for semantic correctness.

## Decisions

### 1. Check stale active paths in index files

**Choice:** Fail if `verification.md`, `tasks.md`, or command artifact pointers include old active paths after archive.
**Rationale:** Active-path leftovers create false active-change reports.
**Implementation:** Allow stale paths only inside diff artifacts whose purpose is historical review.

### 2. Track final preflight order

**Choice:** The final preflight transcript must not be the placeholder text used before staging.
**Rationale:** Placeholder preflight artifacts caused review ambiguity.
**Implementation:** Reject known placeholder text and require transcript begins with `OK:` or `FAIL:`.

## Risks / Trade-offs

**Historical archive noise** → Apply strict freshness only to active changes and newly archived changes touched in the current diff.

## Validation Plan

Fixture tests for archive path rewrite, stale active path failure, and placeholder transcript failure.
