# dogfood-local-connectivity Specification

## Purpose

Defines the Dogfood Local Connectivity capability requirements preserved by Aspen's archived OpenSpec records.
## Requirements
### Requirement: Local node discovery

Dogfood nodes spawned on the same machine MUST discover each other without relay servers or mDNS.

#### Scenario: Local nodes discover peers

- **WHEN** dogfood spawns local nodes with relay servers and mDNS disabled
- **THEN** the nodes MUST discover each other through local cluster discovery

### Requirement: Client connectivity

The dogfood binary's AspenClient MUST connect to spawned nodes within 10 seconds when relay is disabled.

#### Scenario: Client connects without relay

- **WHEN** the dogfood binary starts an AspenClient against spawned local nodes
- **THEN** the client MUST connect within 10 seconds without relay connectivity

### Requirement: Federation trust establishment

Alice and bob clusters MUST successfully exchange AddPeerCluster RPCs and establish bidirectional federation trust.

#### Scenario: Federation trust is established

- **WHEN** alice and bob clusters exchange AddPeerCluster RPCs
- **THEN** bidirectional federation trust MUST be established

### Requirement: Git push through federation

A git push to alice's forge MUST succeed, bob MUST be able to sync the objects via federation protocol, and local dogfood runs MUST expose deterministic evidence for the first failing push boundary when the push does not complete. Local dogfood acceptance pushes MUST avoid transferring unrelated historical commits when proving current-source Forge ingestion and CI trigger acceptance.

#### Scenario: Bob syncs pushed objects

- **WHEN** a git push succeeds against alice's forge
- **THEN** bob MUST sync the pushed objects through the federation protocol

#### Scenario: Local dogfood push failure is bounded and classified

- GIVEN dogfood runs a local same-host cluster with relay servers and mDNS disabled
- WHEN the dogfood `push` stage fails or times out before build, deploy, or verify
- THEN the saved dogfood receipt SHALL identify the first failed push sub-boundary, such as local git invocation, forge receive-pack connection, object ingestion, hook dispatch, CI trigger acceptance, federation/watch publication, or push completion
- AND the failure SHALL include elapsed duration and a redacted operator-visible category without printing credential material

#### Scenario: Local dogfood push uses bounded current-source snapshot

- GIVEN dogfood-local is pushing the current Aspen source into an empty local Forge repo for acceptance
- WHEN the push workspace is prepared
- THEN it MUST contain the committed source tree as a bounded single-commit Git repository rather than the full historical object graph
- AND the push MUST still use the real `git-remote-aspen` Forge path and registered CI watch

### Requirement: Large repo federation sync

A repo with 100+ files in nested directories (3 levels) MUST sync completely from alice to bob. DAG integrity check on bob MUST report 0 missing objects.

#### Scenario: Large repository sync remains complete

- **WHEN** alice federates a repository with 100+ files across nested directories
- **THEN** bob MUST receive a complete sync and report 0 missing DAG objects

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
