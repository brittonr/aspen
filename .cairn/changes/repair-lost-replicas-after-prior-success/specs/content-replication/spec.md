# Content replication: F04 delta

## ADDED Requirements

### Requirement: Historical success does not establish current availability
r[molten.audit_f04.current_availability] Molten MUST derive current replica availability from current admitted observations, not historical success alone.

#### Scenario: Current verified inventory counts
- GIVEN a replica matches the current generation, epochs, manifest, and protected form
- WHEN admitted inventory marks it present and verified
- THEN status counts that replica once.

#### Scenario: Same-epoch loss remains visible
- GIVEN only A remains present after a successful transfer to B in the same generation and epochs
- WHEN reconciliation consumes the fresh inventory and old success
- THEN B does not count as currently verified from history alone.

### Requirement: Loss requires bounded repair or unresolved demand
r[molten.audit_f04.fresh_repair] Molten MUST plan fresh bounded work or explicit unresolved demand after current observations contradict prior success.

#### Scenario: Repair verifies the lost target
- GIVEN A can supply content and B is eligible within the repair budget
- WHEN a new reconciliation observes B absent
- THEN a distinct admitted repair operation requires fresh content verification before success.

#### Scenario: Repair cannot proceed
- GIVEN repair attempts are exhausted or verification fails
- WHEN reconciliation handles the lost replica
- THEN under-replicated demand remains explicit and no historical success fills the deficit.

### Requirement: Replay and rejection preserve state
r[molten.audit_f04.replay_preservation] Molten MUST preserve exact-operation idempotency and MUST reject conflicting evidence without protected effects or durable state mutation.

#### Scenario: Exact replay retains historical meaning
- GIVEN an exact prior operation and its saved result
- WHEN the operation repeats without a new repair intent
- THEN replay returns the prior outcome without a second transfer or a new availability claim.

#### Scenario: Conflicting history cannot commit
- GIVEN prior history conflicts with the operation binding
- WHEN the core evaluates reuse
- THEN the operation rejects and preserves inventory, history, and retention state.

### Requirement: Evidence stays scoped to executed checks
r[molten.audit_f04.validation_claims] Molten MUST retain repository-owned positive and negative tests and MUST distinguish observed repair from historical outcome evidence.

#### Scenario: Regression runs through normal tests
- GIVEN the same-epoch loss fixture from F04
- WHEN normal core and controlled-adapter tests run after the change
- THEN they cover fresh repair and unresolved repair without ignored audit artifacts.

#### Scenario: Historical receipt cannot prove durability
- GIVEN only a historical verified transfer receipt
- WHEN a consumer requests current availability or permanent durability evidence
- THEN the receipt does not establish either claim.
