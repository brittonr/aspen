## Context

The testing harness family can accelerate future extraction work, but reusable defaults must be clearly separated from madsim/network/patchbay/runtime adapters before it becomes a stable public surface. This change is intentionally spec-only at creation time: it defines the implementation/evidence rail without changing Rust code yet.

## Goals / Non-Goals

**Goals:**
- Define the smallest evidence-backed implementation slice for `review-testing-harness-public-api`.
- Preserve current Aspen compatibility while proving the targeted reusable/evidence boundary.
- Leave implementation tasks open for later autonomous drain.

**Non-Goals:**
- Do not raise readiness labels, archive the change, or claim dogfood acceptance before evidence exists.
- Do not broaden scope to unrelated crate families or runtime rewrites.
- Do not use stale storage/Redb guidance as evidence; use live checks.

## Decisions

### 1. Separate spec foundation from implementation evidence

**Choice:** Mark only the spec-foundation task complete and leave all implementation/evidence tasks open.

**Rationale:** The user asked to write OpenSpecs for the targets, not to implement every target in this turn. Active changes should be drainable independently.

**Alternative:** Collapse all targets into one umbrella change. Rejected because each target has a distinct implementation seam and verification rail.

### 2. Evidence must be captured under the active change

**Choice:** Implementation tasks must save command transcripts, fixture metadata, negative checks, and readiness/doc diffs under `openspec/changes/review-testing-harness-public-api/evidence/` or documented repo-local equivalents before tasks are checked off.

**Rationale:** Aspen extraction and dogfood claims need rerunnable, reviewable evidence rather than console-only summaries.

## Risks / Trade-offs

**Scope drift** → Keep each change bounded to its named family and open follow-up changes for unrelated findings.

**False readiness** → Require downstream/negative/compatibility evidence before readiness labels or operator acceptance claims change.
