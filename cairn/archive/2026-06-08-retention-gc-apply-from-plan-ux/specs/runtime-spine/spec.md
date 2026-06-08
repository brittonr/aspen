# Runtime Spine Specification Delta

## Requirements

### Requirement: Retention GC apply from plan
r[molten.retention.gc_apply_from_plan_ux] Molten MUST expose a retention GC apply workflow that requires a stored dry-run plan ref, recomputes the plan from its embedded candidate and destructive evidence immediately before mutation, denies on drift or failed admission before writing destructive retention receipts or tombstones, and emits a canonical apply receipt linking the original plan, recomputed plan, admitted evidence refs, retention receipt ref, and tombstone ref.

#### Scenario: Apply requires unchanged current plan
- GIVEN a passing `retention-gc-plan-v1` artifact and no retention state drift
- WHEN an operator applies retention GC with that plan ref
- THEN Molten recomputes the plan, observes the same plan ref, runs normal destructive admission and retention evaluation, and emits `retention-gc-apply-v1` evidence binding the plan, admission refs, retention receipt, and tombstone refs

#### Scenario: Drift denies before mutation
- GIVEN a `retention-gc-plan-v1` artifact and a later pin, retained dependency, stale admission, or changed remote clearance state
- WHEN an operator applies retention GC with the old plan ref
- THEN Molten emits a denial `retention-gc-apply-v1` receipt, records drift diagnostics, and does not write destructive retention receipts or tombstones

#### Scenario: Plan is not authority at apply time
- GIVEN a passing dry-run plan artifact
- WHEN the apply workflow evaluates destructive retention
- THEN Molten MUST still run normal policy, authority, supporting-evidence, reference-index, remote-GC, and imported remote-clearance admission and MUST NOT treat the plan itself as authority, policy, resource, provenance, transport, execution, source-gate, or remote-GC clearance trust
