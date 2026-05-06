## Context

Aspen foundational type crates are split far enough for a reviewed public API boundary. This change drains the owner/public API review by recording canonical imports, compatibility shims, downstream fixture coverage, negative boundary checks, no-std evidence, and readiness checker output.

## Goals / Non-Goals

**Goals:**
- Define the smallest evidence-backed implementation slice for `review-foundational-types-public-api`.
- Preserve current Aspen compatibility while proving the targeted reusable/evidence boundary.
- Promote the family to `extraction-ready-in-workspace` only after downstream, negative, no-std, and readiness evidence pass.

**Non-Goals:**
- Do not claim publishable/repo-split readiness before license/publication policy exists.
- Do not broaden scope to unrelated crate families or runtime rewrites.
- Do not use stale storage/Redb guidance as evidence; use live checks.

## Decisions

### 1. Evidence-backed readiness promotion

**Choice:** Promote the foundational family to `extraction-ready-in-workspace` once the docs, policy, fixture, negative-boundary evidence, no-default checks, no-std checker, and extraction-readiness checker all pass.

**Rationale:** The stale Redb/table split blocker is already resolved; the remaining decision is whether the current public surfaces have owner-grade evidence. The new fixture imports the direct foundational crates with `default-features = false`, not the root `aspen` shell.

**Alternative:** Keep `workspace-internal` until external publishing work begins. Rejected because the in-workspace extraction criteria are now covered; publication remains blocked separately by license policy.

### 2. Evidence must be captured under the active change

**Choice:** Implementation tasks must save command transcripts, fixture metadata, negative checks, and readiness/doc diffs under `openspec/changes/review-foundational-types-public-api/evidence/` or documented repo-local equivalents before tasks are checked off.

**Rationale:** Aspen extraction and dogfood claims need rerunnable, reviewable evidence rather than console-only summaries.

### 3. Compatibility shims stay explicit

**Choice:** `aspen-traits` remains a portable compatibility surface for re-exported KV request types and narrow traits, while async/runtime traits stay feature-gated; Redb table definitions remain shell/runtime-owned and outside `aspen-storage-types`.

**Rationale:** This preserves current consumers while keeping the portable default graph alloc/no-std-friendly.

## Verification Strategy

- `foundational-types-extraction.classification-records-reusable-surface` and `foundational-types-extraction.classification-records-reusable-surface.evidence` are verified by the updated extraction manifest, inventory, policy, and `verification.md` task coverage.
- `foundational-types-extraction.classification-records-reusable-surface.ready` is verified by `scripts/check-crate-extraction-readiness.rs --candidate-family foundational-types` and the generated readiness report.
- `foundational-types-extraction.live-boundary-evidence` and `foundational-types-extraction.live-boundary-evidence.evidence` are verified by the aspen-core no-std checker, no-default package checks, downstream metadata, and compatibility transcript.
- `foundational-types-extraction.live-boundary-evidence.downstream-fixture` is verified by the downstream fixture test plus a negative forbidden-boundary grep that fails if root `aspen`, Redb, Iroh, Axum, Hyper, Tokio, or Snix runtime shells appear in the portable graph.

## Risks / Trade-offs

**Scope drift** → Keep each change bounded to its named family and open follow-up changes for unrelated findings.

**False readiness** → Require downstream/negative/compatibility evidence before readiness labels or operator acceptance claims change.
