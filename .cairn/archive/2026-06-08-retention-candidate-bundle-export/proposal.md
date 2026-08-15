# retention-candidate-bundle-export

## Summary
Add an operator-facing retention candidate bundle export that packages a read-only explain artifact with its referenced local plan/apply/execute/audit/receipt/tombstone artifacts.

## Motivation
`retention explain` tells operators what local evidence exists for an object. For review or handoff, operators need a bounded directory that contains the explain artifact, a canonical bundle manifest, and the referenced local retention GC/audit artifacts without granting deletion authority.

## Scope
- Add canonical `retention-candidate-bundle-v1` evidence.
- Add `molten test retention bundle-export --root ... --explain ... --out ...`.
- Export `explain.preserves`, `bundle.preserves`, and referenced local GC plan/apply/execute/audit, retention receipt, and tombstone artifacts.
- Add parse/summary/catalog/ledger support and CLI regression coverage.
- Preserve evidence-only safety boundaries.

## Non-Goals
- Granting authority, policy, resource, provenance, transport, execution, source-gate, remote-GC clearance, remote-clearance-import, or deletion trust.
- Replacing retention explain, plan, apply, execute, audit, or destructive admission gates.
- Fetching remote artifacts or packaging non-local evidence.
