## Context

Overclaim findings are preventable if the completion path checks for exact evidence before reporting status.

## Goals / Non-Goals

**Goals:** reduce unsupported completion claims.
**Non-Goals:** block summaries that clearly state uncertainty or omit unsupported claims.

## Decisions

### 1. Detect high-risk claims by phrase families

**Choice:** Check for claims like `clean`, `queue empty`, `all tests pass`, `archived`, and `validated`.
**Rationale:** These are common overclaim categories.
**Implementation:** Done-review scans final response and repo-local evidence records.

### 2. Prefer evidence paths over chat memory

**Choice:** Completion claims about OpenSpec work must cite or have a saved evidence artifact.
**Rationale:** Reviewers need durable proof.
**Implementation:** Encourage final summaries to include artifact paths for nontrivial claims.

## Risks / Trade-offs

**Summary verbosity** → Keep evidence mentions terse; block only unsupported strong claims.

## Validation Plan

Unit tests for claim detection plus an integration fixture with a fake final response.
