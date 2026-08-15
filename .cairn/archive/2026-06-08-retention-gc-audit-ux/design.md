# retention-gc-audit-ux Design

## Overview
The audit workflow is a read-only operator UX over existing retention GC evidence. Given a stored `retention-gc-execute-v1` ref, Molten reads the execution gate, follows its apply ref to `retention-gc-apply-v1`, follows the plan ref to `retention-gc-plan-v1`, and reads the bound retention receipt and tombstone refs when present.

## Evidence Shape
The command emits canonical `retention-gc-audit-v1` evidence with:

- final audit decision,
- subsystem/action/object/class scope,
- plan ref and decision,
- apply ref and decision,
- execution ref and decision,
- retention receipt ref and decision,
- tombstone ref and status,
- sorted diagnostics,
- checks stating that the audit is not authority and normal admission/remote-clearance gates still apply.

The audit decision passes only when the local chain is readable, scope-consistent, and the plan/apply/execution/retention receipt are passing with required tombstone binding for destructive actions.

## Safety Boundaries
`retention-gc-audit-v1` is operator explanation evidence. It MUST NOT authorize deletion or substitute for plan/apply/execute gates, policy, authority, resource, provenance, transport, source-gate, execution, or remote-GC clearance trust. Destructive subsystem commands continue to require matching apply refs and still rerun normal admission before mutation.
