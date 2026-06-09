# retention-gc-audit-ux

## Summary
Add an operator-facing retention GC audit command that starts from a stored execution gate ref and presents the bound plan, apply, execution, retention receipt, and tombstone chain as canonical evidence.

## Motivation
Retention GC now has several deletion-safety artifacts. Operators need one bounded command that explains how a non-dry-run mutation was admitted without treating the audit output as deletion authority.

## Scope
- Add a `retention gc-audit` CLI command.
- Emit canonical `retention-gc-audit-v1` evidence for the plan → apply → execute → receipt → tombstone chain.
- Report missing, mismatched, or denied chain links as diagnostics.
- Preserve existing destructive gates; the audit artifact is explanatory only.

## Non-Goals
- Granting authority, policy, resource, provenance, transport, execution, source-gate, remote-GC clearance, or deletion trust.
- Replacing normal destructive admission, retention evaluation, apply, or execution gates.
- Discovering remote artifacts outside the local retention store.
