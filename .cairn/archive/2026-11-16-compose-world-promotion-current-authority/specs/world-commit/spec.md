# World Commit Delta

## MODIFIED Requirements

### Requirement: Active-head movement and release reservation are locally atomic

r[molten.world_promotion.transaction] Molten MUST update the active head and publish the complete release-reservation set in one local transaction. A failed or stale transaction MUST leave both prior states unchanged. At application time Molten MUST read the content-addressed material of every plan intent through a capability-rooted store, and a missing intent material, or material whose semantic, handler, adapter, or release classification differs from the plan, MUST deny the transaction before any head or reservation change.

#### Scenario: Promotion transaction commits

- GIVEN expected head, authority, policy, intent closure, and reservation identities remain current
- WHEN the local transaction commits
- THEN the candidate MUST become active and every admitted reservation MUST be durable

#### Scenario: Reservation insertion fails

- GIVEN one required reservation cannot enter the transaction
- WHEN promotion publication runs
- THEN the active head MUST remain unchanged and no partial reservation set may become dispatchable

#### Scenario: Intent material is missing or mismatched

- GIVEN a signed intent closure names an intent reference whose material is absent, is not canonical, does not hash to that reference, or declares another adapter than the plan
- WHEN the promotion transaction is applied
- THEN Molten MUST deny the transaction as intent-material unavailable
- AND the active head, the reservation set, and the receipt store MUST remain unchanged

### Requirement: Uncertain outcomes require observation-first reconciliation

r[molten.world_promotion.reconciliation] Molten MUST classify publication and effect observations as not published, published, unknown, or conflicting before retry decisions. Unknown or conflicting outcomes MUST NOT trigger a blind new logical operation. Operator deny, retry planning, and abandon MUST each recheck a verified operator authority grant that is admitted at the reservation's generation and MUST bind that grant's branch, authority, and policy to the immutable original plan that the store recorded as committed. Deny MUST NOT relabel an in-flight or uncertain release as denied. Retry planning MUST keep its explicit duplicate-risk acknowledgement, abandon MUST keep its explicit unknown-outcome acknowledgement, and neither MUST mint a replacement logical identity.

#### Scenario: Local commit result is unknown

- GIVEN the transaction returned no reliable completion result
- WHEN recovery begins
- THEN Molten MUST observe the durable operation identity before planning any repeat mutation

#### Scenario: External acknowledgment was lost

- GIVEN an effect may have completed but no acknowledgment is durable
- WHEN reconciliation runs
- THEN Molten MUST retain an uncertain state unless an admitted observation resolves it

#### Scenario: Operator authority is absent, stale, revoked, or foreign

- GIVEN an uncertain attempt or a committed reservation
- WHEN an operator requests deny, retry planning, or abandon without a verified grant, with a grant at another generation, with a grant that is not admitted, or with a grant for another branch, authority, or policy than the original plan
- THEN Molten MUST deny the request
- AND the reservation and attempt records MUST remain unchanged

#### Scenario: Operator request names another plan

- GIVEN a request document whose identity fields differ from the committed promotion of its logical operation, or whose logical operation was never committed
- WHEN deny, retry planning, or abandon rebuilds the original plan
- THEN Molten MUST deny the request rather than plan against a replacement identity

#### Scenario: Deny cannot hide an attempted effect

- GIVEN a reservation is in flight, uncertain, or already has an observed effect
- WHEN an admitted operator requests deny
- THEN Molten MUST reject the mutation and preserve its existing outcome
- AND unknown effects still require observation-first reconciliation or acknowledged abandon.

