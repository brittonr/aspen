# runtime-spine Spec Delta

## Requirements

### Requirement: Retention GC execution gates
r[molten.retention.gc_execution_gates] Molten MUST require a matching passing retention GC apply receipt before non-dry-run destructive subsystem mutation and MUST emit canonical per-candidate execution gate evidence without treating plans, apply receipts, or execution gate receipts as authority, policy, resource, provenance, transport, execution, source-gate, or remote-GC clearance trust.

#### Scenario: Matching apply gates physical mutation
- GIVEN a passing retention GC plan and apply receipt for a ledger, chunk, or cache candidate
- WHEN a non-dry-run subsystem GC or invalidation operation attempts physical mutation for that candidate
- THEN Molten verifies the apply scope, plan binding, retention receipt, and tombstone refs, emits `retention-gc-execute-v1`, and only mutates after normal destructive admission and retention evaluation still pass

#### Scenario: Missing or wrong apply denies before mutation
- GIVEN a destructive subsystem candidate with no apply ref or an apply ref for a different object, class, action, or subsystem
- WHEN a non-dry-run GC or invalidation operation runs
- THEN Molten emits denial diagnostics and leaves the selected content or cache entry readable

#### Scenario: Fresh retention drift after apply still blocks execution
- GIVEN a passing apply receipt followed by a new pin, retained dependency, stale admission, or remote clearance change
- WHEN subsystem execution evaluates the candidate
- THEN Molten denies before physical mutation even if the apply receipt itself remains parseable
