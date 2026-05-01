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

## Verification Strategy

- `openspec-governance.verification-index-freshness`: fixture suite covers fresh verification indexes, stale archived active paths, saved-diff exceptions, placeholder preflight artifacts, and generated-before-final-diff artifacts.
- `openspec-governance.verification-index-freshness.fresh-index-passes`: fresh fixture with current archive paths and final `OK:` preflight transcript must pass.
- `openspec-governance.verification-index-freshness.stale-active-path-fails`: archived index fixture citing `openspec/changes/<name>/` must fail with the stale path.
- `openspec-governance.verification-index-freshness.saved-diff-may-contain-historical-paths`: saved diff artifact fixture may contain active-path strings while current `verification.md` remains archive-relative.
- `openspec-governance.verification-index-freshness.placeholder-preflight-fails`: cited preflight artifact containing placeholder text must fail.
- `openspec-governance.verification-index-freshness.generated-before-final-diff-fails`: final artifact timestamp older than changed files must fail.

## Validation Plan

Fixture tests for archive path rewrite, stale active path failure, and placeholder transcript failure.
