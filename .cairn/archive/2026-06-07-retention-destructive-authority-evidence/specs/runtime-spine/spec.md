# Runtime Spine Delta: retention destructive authority evidence

### Requirement: Explicit destructive retention evidence
r[molten.retention.destructive_evidence_inputs] Molten MUST require destructive subsystem retention evaluations to accept explicit requester, policy, authority, evidence, retained-reference, remote-reference, and reference-index completeness inputs rather than relying on mutable names or synthesized trust.

#### Scenario: Destructive caller supplies evidence inputs
- GIVEN a ledger GC, chunk GC, or cache invalidation candidate
- WHEN the subsystem evaluates retention eligibility
- THEN the retention evaluation binds the explicit requester, policy refs, evidence refs, retained refs, remote refs, and reference-index completeness supplied by the caller

r[molten.retention.apply_requires_authority] Molten MUST deny apply-mode destructive candidates when requester, policy, authority, or supporting evidence refs are missing.

#### Scenario: Missing authority denies before removal
- GIVEN an apply-mode destructive candidate without delete authority evidence
- WHEN the subsystem attempts removal or tombstoning
- THEN the operation emits denial evidence and does not remove or tombstone the object

r[molten.retention.reference_index_plumbing] Molten MUST pass retained refs, remote refs, and reference-index completeness through destructive subsystem retention checks so incomplete proofs fail closed.

#### Scenario: Remote uncertainty blocks apply
- GIVEN a destructive candidate with unresolved remote cache refs or an incomplete reference index
- WHEN apply-mode GC or invalidation evaluates the candidate
- THEN deletion or tombstoning is denied before mutation

### Requirement: Destructive retention evidence receipts
r[molten.retention.cli_evidence_flags] Molten MUST expose operator-facing CLI flags for destructive retention requester, policy, authority, evidence, retained, remote, and reference-index completeness inputs.

#### Scenario: CLI surfaces missing evidence
- GIVEN a destructive CLI command without required retention evidence flags
- WHEN candidates are selected for apply-mode mutation
- THEN the command emits a denial receipt and reports the missing evidence diagnostics

r[molten.retention.evidence_summary_receipts] Molten MUST bind retention evidence summaries in subsystem GC and invalidation receipts without treating those summaries as authority grants.

#### Scenario: Receipt records evidence summary
- GIVEN a destructive subsystem decision
- WHEN the subsystem receipt is emitted
- THEN it records the retention receipt refs and the retention evidence inputs that informed the decision

r[molten.retention.destructive_evidence_tests] Molten MUST test fail-closed destructive retention evidence behavior for missing authority, missing policy, missing evidence, incomplete indexes, retained refs, and remote uncertainty.

#### Scenario: Evidence tests leave content intact
- GIVEN destructive candidates with incomplete or missing retention evidence
- WHEN subsystem cleanup runs
- THEN tests verify denial receipts are auditable and selected content remains intact
