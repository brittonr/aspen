## MODIFIED Requirements

### Requirement: Relay-disabled dogfood clients fail fast on missing direct routes [r[dogfood-local-connectivity.direct-only-route-preflight]]

When dogfood runs with relay disabled, every Aspen client path that is expected to contact a ticket peer directly MUST verify that it has a usable direct route source before entering long-running stage waits.

#### Scenario: Missing direct address fails before CI timeout [r[dogfood-local-connectivity.direct-only-route-preflight.missing-address]]

- GIVEN dogfood is running in relay-disabled or direct-only mode
- AND a ticket or selected peer lacks direct socket addresses usable by the client endpoint
- WHEN the dogfood runner is about to start a long health, push, or CI wait loop
- THEN the runner MUST fail before the generic stage timeout
- AND the failure MUST identify the condition as direct-only route loss rather than pipeline-pending, worker-readiness, or CI execution timeout
- AND the failure MUST include the peer identifier and route-source summary without printing the full ticket or credential material

#### Scenario: Address lookup warning is treated as route loss [r[dogfood-local-connectivity.direct-only-route-preflight.address-lookup-warning]]

- GIVEN relay, mDNS, DNS, and other external discovery are intentionally disabled for a dogfood proof
- AND Iroh reports `No address lookup configured` for the peer that the current stage is waiting on
- WHEN no direct address has been registered for that peer
- THEN the dogfood/client layer MUST classify the failure as missing direct route configuration
- AND it MUST stop retrying within the bounded direct-route preflight window

### Requirement: Ticket direct addresses remain available to later dogfood RPC clients [r[dogfood-local-connectivity.ticket-direct-address-retention]]

Dogfood connection helpers MUST preserve or register ticket-derived direct addresses so later clients created for CI polling or receipt publication can reach the same peer route used during initial node health.

#### Scenario: Initial health route is reused by CI polling [r[dogfood-local-connectivity.ticket-direct-address-retention.ci-polling]]

- GIVEN initial dogfood health succeeds by connecting to a ticket peer through direct socket addresses
- AND the same run later creates or reuses a client for CI pipeline polling
- WHEN relay is disabled
- THEN the CI polling client MUST have an address-book or equivalent route source containing the ticket-derived direct address for that peer
- AND the client MUST NOT depend on relay, DNS, mDNS, pkarr, or ambient address lookup to rediscover the peer

#### Scenario: Missing retained route is regression-tested [r[dogfood-local-connectivity.ticket-direct-address-retention.negative-test]]

- GIVEN a test ticket peer has no direct addresses and relay/discovery are disabled
- WHEN dogfood constructs the CI wait client or runs its route preflight
- THEN the test MUST observe the targeted direct-only route-loss error
- AND the test MUST NOT wait for the full CI-stage timeout
