## Context

Aspen needs an operator-grade receipt proving the current pushed head still satisfies the self-hosting loop instead of relying on historical dogfood evidence from older commits. This change is intentionally spec-only at creation time: it defines the implementation/evidence rail without changing Rust code yet.

## Goals / Non-Goals

**Goals:**
- Define and drain the smallest evidence-backed implementation slice for `dogfood-current-head-acceptance-receipt`.
- Preserve current Aspen compatibility while proving the targeted reusable/evidence boundary.
- Save redacted current-head dogfood receipt, readback, diagnose, and blocker evidence under the change.

**Non-Goals:**
- Do not claim full dogfood acceptance if the run gates before deploy/verify/publish_receipt.
- Do not broaden scope to unrelated crate families or runtime rewrites.
- Do not use stale storage/Redb guidance as evidence; use live checks.

## Decisions

### 1. Commit-bound receipts are acceptance evidence

**Choice:** Treat the dogfood receipt as the durable operator record only when it records the checked-out git commit, schema, run id, stage outcomes, timings, artifact identifiers, and failure category when any.

**Rationale:** Current-head evidence must be auditable after logs rotate and must not depend on the operator remembering which commit was run.

**Alternative:** Keep a console transcript only. Rejected because raw logs are hard to compare, easy to leak, and do not provide a stable operator readback surface.

### 2. Evidence must be captured under the active change

**Choice:** Save the redacted dogfood log, redacted JSON receipt copy, receipt show output, receipt diagnose output, and verification matrix under `openspec/changes/dogfood-current-head-acceptance-receipt/evidence/` or `verification.md` before tasks are checked off.

**Rationale:** Aspen extraction and dogfood claims need rerunnable, reviewable evidence rather than console-only summaries.

## Verification Strategy

- Cover `r[dogfood-evidence.current-head-receipt-durable]` and `r[dogfood-evidence.current-head-receipt-durable.evidence]` by running the current-head dogfood command with explicit locally built binaries and fresh `/tmp/aspen-dogfood*` directories.
- Cover `r[dogfood-evidence.receipt-readback-operator-evidence]` and `r[dogfood-evidence.receipt-readback-operator-evidence.evidence]` by committing only redacted evidence: JSON receipt copy, dogfood log, receipt show output, and receipt diagnose output.
- If the loop gates before deploy/verify/publish_receipt, capture the local reproduction command and failure category instead of claiming success.
- Run `openspec validate dogfood-current-head-acceptance-receipt --strict`, `scripts/openspec-preflight.sh dogfood-current-head-acceptance-receipt`, and `git diff --check` before archiving.

## Risks / Trade-offs

**Scope drift** → Keep each change bounded to its named family and open follow-up changes for unrelated findings.

**False readiness** → Require downstream/negative/compatibility evidence before readiness labels or operator acceptance claims change.
