# retention-candidate-explain-ux

## Summary
Add a read-only retention candidate explain command that starts from an object ref and lists known retention evidence before an operator chooses a destructive workflow.

## Motivation
Retention GC now emits plans, applies, executions, audits, receipts, clearances, and tombstones. Operators need one local explanation artifact that answers "what does Molten already know about this candidate?" without creating deletion authority or mutating retained content.

## Scope
- Add canonical `retention-candidate-explain-v1` evidence.
- Add `molten test retention explain` with object-ref plus optional object-kind/class/action/subsystem filters.
- List local pins, admissions, remote clearances/imports, plans, applies, executions, audits, retention receipts, and tombstones.
- Store audit artifacts under the retention root when `gc-audit` runs so later explain commands can find known audits.
- Add unit and CLI regression coverage plus docs.

## Non-Goals
- Granting deletion authority, policy, resource, provenance, transport, execution, source-gate, remote-GC clearance, or remote clearance import trust.
- Replacing GC plan/apply/execute gates or destructive retention admission.
- Fetching remote evidence outside the local retention root.
