## Context

Blob/castore/cache is high-value for self-hosting and cache infrastructure; current manifests say key couplings are moved/gated but readiness still lacks complete fixtures, checker updates, and compatibility evidence. This change is intentionally spec-only at creation time: it defines the implementation/evidence rail without changing Rust code yet.

## Goals / Non-Goals

**Goals:**
- Define the smallest evidence-backed implementation slice for `complete-blob-castore-cache-readiness`.
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

**Choice:** Implementation tasks must save command transcripts, fixture metadata, negative checks, and readiness/doc diffs under `openspec/changes/complete-blob-castore-cache-readiness/evidence/` or documented repo-local equivalents before tasks are checked off.

**Rationale:** Aspen extraction and dogfood claims need rerunnable, reviewable evidence rather than console-only summaries.

## Risks / Trade-offs

**Scope drift** → Keep each change bounded to its named family and open follow-up changes for unrelated findings.

**False readiness** → Require downstream/negative/compatibility evidence before readiness labels or operator acceptance claims change.

## Verification Strategy

- Requirement coverage: `blob-castore-cache-extraction.promotion-requires-complete-evidence`, `blob-castore-cache-extraction.promotion-requires-complete-evidence.evidence`, `blob-castore-cache-extraction.adapter-paths-explicit`, and `blob-castore-cache-extraction.adapter-paths-explicit.evidence`.
- Treat `scripts/check-crate-extraction-readiness.rs --candidate-family blob-castore-cache` as the readiness-policy oracle for `aspen_blob`, `aspen_castore`, and `aspen_cache`.
- Prove reusable-consumer compatibility with standalone downstream fixtures for blob and cache/castore imports, plus metadata/forbidden-dependency artifacts.
- Prove workspace compatibility with focused package checks: `aspen-blob` no-default/replication, `aspen-cache` no-default and `kv-index`, and the `aspen-castore` circuit-breaker regression.
- When dependency lockfile revisions change, rerun the `aspen-core --no-default-features` boundary checker and update dependency-review notes in the same slice.
- Close the change only after `openspec validate`, repo-local `scripts/openspec-preflight.sh`, and whitespace checks pass with staged evidence.
