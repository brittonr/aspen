## ADDED Requirements

### Requirement: Peer promotions require explicit preflight and apply
r[molten.peer_promotion.preflight_apply] Molten MUST separate peer promotion preflight from promotion apply so operators can inspect requested role deltas, diagnostics, missing evidence, and expected session changes before mutating peer-session state.

#### Scenario: Preflight does not mutate session
- GIVEN a peer requests promotion from read-only subscriber to scoped publisher
- WHEN promotion preflight passes
- THEN Molten emits a preflight receipt and readback summary
- AND the peer session remains read-only until an explicit apply operation passes.

### Requirement: Subscriber write upgrade requires promotion
r[molten.peer_promotion.subscriber_upgrade] Molten MUST require an explicit passing promotion grant and apply receipt before a subscriber or read-only peer gains publish, assert, retract, relay, sync-import, execution, or mutation capabilities.

#### Scenario: Subscriber cannot publish before apply
- GIVEN a subscriber has a passing promotion preflight but no promotion apply receipt
- WHEN it attempts to publish to the target topic
- THEN the write denies
- AND diagnostics state that promotion has not been applied.

### Requirement: Promotion diagnostics explain role deltas
r[molten.peer_promotion.diagnostics] Molten SHOULD render diagnostics that identify current roles, requested roles, admitted and denied capability deltas, missing promotion authority, expiry/revocation, approval requirements, and next operator actions.

#### Scenario: Missing promotion authority names next step
- GIVEN a peer requests promotion to a scoped publisher role without a promotion grant
- WHEN diagnostics render the request
- THEN they report current subscriber role, requested publisher role, missing promotion authority, and the required grant/import step.

### Requirement: Promotion validation is reproducible
r[molten.peer_promotion.validation] Molten SHOULD validate promotion and demotion with focused promotion tests, peer-session tests, subscriber/read-only tests, authority tests, consensus boundary tests, formatting, and Cairn validation before archiving.

#### Scenario: Subscriber write-upgrade regression fails validation
- GIVEN a regression allows a subscriber to publish without promotion apply
- WHEN focused promotion validation runs
- THEN the negative subscriber upgrade test fails
- AND the change cannot complete until promotion apply is required again.
