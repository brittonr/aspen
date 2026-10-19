# Bound lifecycle probe status-reference collection

## Why

The original bound F09/F10 full Octet catalog marks F1922/F1923 at `src/lifecycle/parts/mod/p003/body.rs:109`. Public production `phases::validate_probe_results` accepts caller-provided probe results, each optionally holding a status-condition reference, and previously cloned every such reference into an unbounded returned vector. This entry point had no count or byte guard; the isolated exact/+1 regression failed before the source fix.

## Proposed change

Reuse the already declared lifecycle `MAX_REFS = 1_024` envelope to reject a 1,025th probe before any status-reference copy. Preserve admission and returned reference order at 1,024, existing evidence/generation checks for admitted input, and a deterministic invalid-harness error for excess input.

## Ownership and non-claims

This isolated source owner concerns a public lifecycle API, not cluster-harness input, resource deletion, or world-head storage. A disposable compiled external caller exercises the real library API. The change-local runtime-spine delta is reviewable without modifying accepted specs; scoped Octet is not full F09/F10 strict-ci acceptance, and no waiver, pin change, commit, sync, or archive is claimed.
