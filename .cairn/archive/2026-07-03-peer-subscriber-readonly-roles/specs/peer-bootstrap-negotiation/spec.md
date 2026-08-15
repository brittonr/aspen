## ADDED Requirements

### Requirement: Subscriber roles are scoped read capabilities
r[molten.peer_subscriber.role_model] Molten MUST model subscriber and read-only peer roles as scoped, attenuated read capabilities that bind projection kind, allowed scope, egress policy, redaction profile, resource limits, expiry, revocation, policy refs, resource refs, and evidence refs.

#### Scenario: Subscriber role admits observe-only scope
- GIVEN a peer session has a subscription grant for one remote dataspace topic projection
- WHEN the peer requests read access to that projection
- THEN Molten admits only the declared projection scope
- AND the role does not admit publish, assert, retract, control, execution, retention, import, relay, or authority-delegating operations.

### Requirement: Subscription grants and projection receipts are canonical
r[molten.peer_subscriber.subscription_grant] Molten MUST define canonical subscription grant and projection receipt records for read-only peer delivery decisions.

#### Scenario: Projection receipt binds filter decision
- GIVEN a subscriber receives a redacted service-status projection
- WHEN Molten emits the projection receipt
- THEN the receipt binds the subscription grant ref, peer/session refs, source ref, projection ref, egress policy ref, redaction decision, resource consumption, replayability flag, and diagnostics.

### Requirement: Read-only still requires authority
r[molten.peer_subscriber.read_requires_authority] Molten MUST require explicit read authority, policy, resource, and egress admission before delivering subscription data to a read-only peer.

#### Scenario: Missing read grant denies delivery
- GIVEN a peer is connected and admitted for transport but has no subscription grant for a topic
- WHEN the peer requests that topic projection
- THEN delivery denies before data egress
- AND diagnostics state that read-only access still requires explicit read authority.

### Requirement: Subscriber grants cannot upgrade to writes
r[molten.peer_subscriber.no_write_upgrade] Molten MUST deny attempts to use subscriber or read-only grants for publish, assert, retract, node-control mutation, job execution, sync import, retention clearance, authority delegation, or destructive operations.

#### Scenario: Subscriber publish attempt denies
- GIVEN a peer has a read-only subscription grant for a topic
- WHEN the peer attempts to publish or assert into that topic
- THEN Molten denies before routing the write
- AND diagnostics identify the grant as read-only.

### Requirement: Subscriber relay and republish are separate scopes
r[molten.peer_subscriber.no_relay_republish] Molten MUST deny relay, republish, cache export, and transitive subscription from read-only peers unless the subscription grant explicitly includes those attenuated scopes.

#### Scenario: Read-only peer cannot republish inventory
- GIVEN a peer receives catalog inventory under a read-only subscription grant
- WHEN it attempts to republish that inventory as an authority-bearing announcement
- THEN Molten denies republish authority
- AND the original projection receipt remains evidence-only.

### Requirement: Subscriber roles have positive and negative tests
r[molten.peer_subscriber.positive_negative_tests] Molten SHOULD include positive subscriber projection tests and negative tests for missing read authority, egress denial, stale grants, write-upgrade attempts, unauthorized republish, read-only sync import, and Raft learner confusion.

#### Scenario: Stale grant fixture denies
- GIVEN a subscriber grant is expired or revoked
- WHEN projection delivery validation runs
- THEN the decision is deny
- AND diagnostics identify the stale or revoked grant.
