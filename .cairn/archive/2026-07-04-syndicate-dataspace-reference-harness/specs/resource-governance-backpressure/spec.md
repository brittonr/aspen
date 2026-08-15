## ADDED Requirements

### Requirement: Syndicate flow-control observations become Molten resource evidence
r[molten.syndicate_dataspace.flow_control_receipts] Molten SHOULD record Syndicate account, debt, loaned-item, fanout, and repayment observations as canonical Molten resource or backpressure evidence where the Syndicate reference harness uses them. Decisions MUST be derived from explicit bounds and recorded observations, not host scheduler timing.

#### Scenario: Fanout debt is bounded deterministically
- GIVEN a reference harness input that routes one incoming assertion to multiple observers
- WHEN the fanout would exceed the declared resource budget
- THEN Molten emits throttle or deny resource evidence with account/debt observations
- AND committed dataspace state follows the deterministic resource decision.

#### Scenario: Scheduler timing cannot change resource decision
- GIVEN the same canonical harness input, budget, account observations, and repayment sequence
- WHEN host thread scheduling differs between runs
- THEN the Molten resource decision and receipt refs remain unchanged
- OR the evidence is marked diagnostic-only because required observations were not recorded.
