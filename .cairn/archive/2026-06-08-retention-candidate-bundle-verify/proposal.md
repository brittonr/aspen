# retention-candidate-bundle-verify

## Summary
Add a read-only verification workflow for exported retention candidate bundles.

## Motivation
`retention bundle-export` produces a review/handoff directory, but operators need a deterministic way to verify that a received bundle still matches its manifest and packaged artifacts before review.

## Scope
- Add canonical `retention-candidate-bundle-verify-v1` evidence.
- Add `molten test retention bundle-verify --bundle ... [--receipt-out ...]`.
- Verify `bundle.preserves`, `explain.preserves`, grouped artifact files, artifact refs, and canonical hashes.
- Detect missing, tampered, duplicate, and unreferenced packaged files.
- Preserve evidence-only safety boundaries.

## Non-Goals
- Importing bundle contents into a retention store.
- Granting authority, policy, resource, provenance, transport, execution, source-gate, remote-GC clearance, remote-clearance-import, or deletion trust.
- Replacing retention explain, plan, apply, execute, audit, or destructive admission gates.
