## Context

This change began as a Tiger Style audit refresh against the 2026-04-09 baseline. During implementation, user requested that the remediation backlog start immediately and continue through completion. The result is a combined audit-and-remediation slice:

- audit artifacts remain the durable baseline input
- production refactors remove the named hotspots from that baseline
- scanner regressions are fixed in the same slice because they changed backlog ordering

## Goals / Non-Goals

**Goals**

- Preserve the refreshed audit report and baseline comparison.
- Remove the named remediation targets from the Tiger Style hotspot list.
- Keep refactors bounded by extracting deterministic helpers and table-driven metadata.
- Add direct verification artifacts for each completed remediation phase.
- Fix scanner heuristics that produced a false parse error and a comment-only ambient-time hit.

**Non-Goals**

- Eliminate every remaining Tiger Style hotspot in the repo.
- Rewrite the scanner as a full Rust parser.
- Change external Aspen protocols or feature flags.
- Hide the original audit input; the report still records the pre-remediation backlog scan.

## Decisions

### 1. Keep audit evidence and remediation in one active change

**Choice:** Update the active change to describe both the baseline audit refresh and the completed remediation slice.

**Rationale:** User asked to begin the backlog immediately. Splitting after implementation would create unnecessary artifact churn while the working tree already contains both the audit evidence and the code changes.

### 2. Refactor monoliths by extracting bounded helpers, not by rewriting behavior

**Choice:** Large functions were split into smaller phases/helpers that preserve the existing request flow.

**Rationale:** Tiger Style hotspot removal should not change distributed semantics. Helper extraction keeps behavior local, reviewable, and regression-testable.

### 3. Replace duplicated request metadata matches with one table

**Choice:** `ClientRpcRequest::{variant_name, required_app}` now delegate to one generated metadata module.

**Rationale:** Two giant hand-maintained matches had to stay in lockstep. A single table removes drift risk and keeps both methods bounded.

### 4. Keep trust/secrets logic explicit about thresholds and epochs

**Choice:** Trust rotation and secrets-at-rest changes extract planning helpers and assert threshold/epoch invariants close to the orchestration sites.

**Rationale:** Those hotspots came from recent trust work. The right fix is clearer planning structure, not hiding quorum logic.

### 5. Treat scanner regressions as production-priority because they distort the backlog

**Choice:** Fix the lifetime-vs-char-literal parser bug and strip comments before ambient-time scans.

**Rationale:** False parse errors and comment-only hits make the audit report misleading. Fixing the tool is part of completing the remediation slice.

## Risks / Trade-offs

**Large review surface** -> touching many files raises review cost.
**Mitigation:** save targeted scan/test artifacts per phase and keep each refactor local to named hotspots.

**Refactor regressions** -> helper extraction can drop side effects.
**Mitigation:** keep characterization tests, targeted cargo checks, and post-remediation hotspot scans. This change also restores subprocess cache-proxy shutdown in the CI executor fallback path.

**Audit numbers can drift again later** -> full-repo counts may change after this lands.
**Mitigation:** report keeps the baseline scan as input and separately records the post-remediation targeted verification artifacts.
