# World commit: promotion identity delta

## ADDED Requirements

### Requirement: Promotion records carry four identity roles
r[molten.promotion_identity.roles] Molten promotion records MUST carry a logical operation identity, a canonical request identity, an attempt identity, and a generation/fencing identity, and the logical operation identity MUST be persisted before the first effect submission.

#### Scenario: Roles survive durable reopen
- GIVEN a promotion record committed with all four identity roles
- WHEN the store reopens from the same storage
- THEN every role reads back exactly as committed.

#### Scenario: Logical identity persists before submission
- GIVEN a new promotion operation at the origin
- WHEN the first submission is prepared
- THEN the logical operation identity is already durable.

### Requirement: Logical identity binds its canonical request
r[molten.promotion_identity.request_binding] Molten MUST reject a retry that reuses a logical operation identity with an incompatible canonical request identity, without state mutation.

#### Scenario: Incompatible parameters are rejected
- GIVEN a committed logical operation with its canonical request identity
- WHEN a request with the same logical identity but different parameters arrives
- THEN the transition rejects with a typed error and durable state is unchanged.

#### Scenario: Matching parameters replay
- GIVEN a committed logical operation and a retry with identical parameters
- WHEN the retry is evaluated
- THEN it is treated as a replay of the original operation.

### Requirement: Payload equality never merges operations
r[molten.promotion_identity.no_payload_merge] Molten MUST keep two distinct logical operations with identical canonical payloads separate.

#### Scenario: Identical payloads remain distinct
- GIVEN two operations with different logical identities and identical payload hashes
- WHEN both are committed
- THEN each retains its own outcome and neither substitutes the other's result.

### Requirement: Logical identity survives restart and uncertainty
r[molten.promotion_identity.restart_binding] Molten MUST preserve the original logical operation identity across `OutcomeUnknown`, process restart, and retry, and MUST NOT mint a replacement logical identity for an unresolved operation.

#### Scenario: Lost response keeps the original identity
- GIVEN a committed reservation whose response was lost after a possible external effect
- WHEN the process restarts on the same storage and the caller retries with the original logical identity
- THEN the retry resolves against the original operation and retains the recorded uncertainty.

#### Scenario: Restart does not re-mint identity
- GIVEN an unresolved operation at shutdown
- WHEN the node restarts and resumes the promotion path
- THEN the shell does not substitute a new logical identity for the stored operation.

### Requirement: Validation limits identity evidence
r[molten.promotion_identity.validation] Molten MUST retain positive and negative repository tests for the identity contract and MUST NOT claim exactly-once execution for external destinations.

#### Scenario: Contract regression runs after reopen
- GIVEN the full commit, lost-response, restart, retry sequence
- WHEN normal core and adapter tests execute it
- THEN the original identity resolves and uncertainty or the retained outcome is returned.

#### Scenario: External effects keep the uncertainty boundary
- GIVEN a destination outside the authoritative transactional state machine
- WHEN the promotion path reports outcomes
- THEN it preserves uncertainty rather than claiming exactly-once execution.
