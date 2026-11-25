# Coordination deltas

## ADDED Requirements

### Requirement: Coordination commits real fencing token refs, valid assertion refs, explicit lock/election expiry, and exact control groups
r[molten.coordination.truthful_tokens_assertions_and_expiry] Committed `LockState` and `ElectionState` MUST persist the actual `commit_receipt_ref`-bound `FencingToken.token_ref` rather than placeholder refs; `receipt.dataspace_assertion_refs` MUST match the emitted `CoordinationStatusAssertion`; locks and elections MUST support explicit expiry/resignation transitions; and `new_coordination_runtime` MUST bind the caller's `control_group_ref` rather than silently substituting a hardcoded fixture manifest.

#### Scenario: Persisted lock state matches issued FencingToken and receipt assertion ref
- GIVEN a successful lock acquisition
- WHEN status is read back and compared to the returned `FencingToken` and receipt
- THEN persisted `token_ref` MUST equal `FencingToken.token_ref` and `receipt.dataspace_assertion_refs` MUST reference the actual emitted assertion.

#### Scenario: Expired lock or resigned election unlocks coordination key
- GIVEN a lock holder crashes past its lease expiry or an elected leader resigns
- WHEN expiry or resignation runs
- THEN another eligible contender MUST be able to acquire the lock or election with a strictly advancing fencing token.
