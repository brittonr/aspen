## ADDED Requirements

### Requirement: Publish KV prefix for federation

The system SHALL allow cluster administrators to declare KV prefixes as available for federation. Each published prefix SHALL have an access policy (Public or TokenRequired) and an optional `ResourcePolicy` governing verification and fork detection.

#### Scenario: Publish a prefix as public

- **WHEN** an administrator publishes prefix `_sys:nix-cache:narinfo:` with access policy `Public`
- **THEN** any cluster presenting a valid credential with `Read` capability for that prefix SHALL be able to subscribe
- **AND** the publication SHALL be stored in KV at `_sys:fed:pub:{prefix_hash}`

#### Scenario: Publish a prefix as token-required

- **WHEN** an administrator publishes prefix `_forge:repos:` with access policy `TokenRequired`
- **THEN** only clusters presenting a credential with an explicit `Read` capability covering that prefix SHALL be able to subscribe
- **AND** clusters without a matching credential SHALL be rejected with an authorization error

#### Scenario: Publish with resource policy

- **WHEN** an administrator publishes prefix `_forge:repos:aspen:` with a resource policy requiring quorum of 2 and delegate signatures
- **THEN** the resource policy SHALL be stored alongside the publication
- **AND** sync operations for that prefix SHALL enforce the policy

### Requirement: Subscribe to remote prefix

The system SHALL allow clusters to subscribe to a published prefix on a remote cluster. A subscription SHALL specify the source cluster, target prefix, sync mode, and a valid credential.

#### Scenario: Create a subscription with periodic sync

- **WHEN** an operator creates a subscription to `_sys:nix-cache:narinfo:` on Cluster A with sync mode `Periodic(60s)`
- **THEN** the system SHALL poll Cluster A for new/changed entries under that prefix every 60 seconds
- **AND** the subscription state SHALL be stored in KV at `_sys:fed:sub:{source_key}:{prefix_hash}`

#### Scenario: Create a subscription with gossip-triggered sync

- **WHEN** an operator creates a subscription to `_forge:repos:aspen:refs:` on Cluster A with sync mode `OnGossip`
- **THEN** the system SHALL sync when it receives a `ResourceUpdated` gossip event matching the prefix from Cluster A
- **AND** the system SHALL NOT poll on a fixed interval

#### Scenario: Subscription rejected for insufficient credentials

- **WHEN** a cluster attempts to subscribe to prefix `_forge:repos:` on Cluster A
- **AND** the presented credential only has `Read{prefix: "_sys:nix-cache:"}`
- **THEN** Cluster A SHALL reject the subscription with an `Unauthorized` error
- **AND** no data SHALL be transferred

### Requirement: Sync KV entries for subscribed prefix

The system SHALL synchronize KV entries matching a subscribed prefix from the source cluster to the subscribing cluster. Associated blobs referenced by KV entries SHALL be fetchable via iroh-blobs.

#### Scenario: Initial sync of prefix

- **WHEN** a subscription is first activated for prefix `_sys:nix-cache:narinfo:`
- **THEN** the system SHALL fetch all KV entries under that prefix from the source cluster
- **AND** store them in the local KV store under the same keys
- **AND** blob hashes referenced in entry values SHALL be resolvable via iroh-blobs from the source

#### Scenario: Incremental sync after initial

- **WHEN** a sync runs for a subscription that has previously synced
- **THEN** the system SHALL fetch only entries changed since the last sync cursor
- **AND** the cursor SHALL be updated to reflect the sync position

#### Scenario: Sync bounded by resource limits

- **WHEN** a sync request returns more entries than `MAX_OBJECTS_PER_SYNC` (1,000)
- **THEN** the system SHALL paginate using the cursor from the response
- **AND** subsequent pages SHALL be fetched until all entries are synced or a configurable max per-sync limit is reached

### Requirement: Subscription persistence

The system SHALL persist subscription state across node restarts. Subscription configuration, credentials, sync cursors, and last-sync timestamps SHALL survive cluster restarts.

#### Scenario: Subscription survives restart

- **WHEN** a node restarts
- **THEN** all active subscriptions SHALL resume syncing from their last cursor position
- **AND** credentials SHALL be re-validated (expiry check) before resuming

#### Scenario: Expired credential on restart

- **WHEN** a node restarts and a subscription's credential has expired during downtime
- **THEN** the subscription SHALL enter a `NeedsRefresh` state
- **AND** the system SHALL attempt to refresh the token from the source cluster
- **AND** sync SHALL NOT proceed until a valid credential is obtained

### Requirement: Unpublish and unsubscribe

The system SHALL allow administrators to remove published prefixes and subscriptions.

#### Scenario: Unpublish a prefix

- **WHEN** an administrator unpublishes prefix `_sys:nix-cache:narinfo:`
- **THEN** new subscription requests for that prefix SHALL be rejected
- **AND** existing active subscriptions SHALL be notified via a `ResourceRemoved` gossip event
- **AND** the publication record SHALL be removed from KV

#### Scenario: Unsubscribe from a prefix

- **WHEN** an operator removes a subscription to Cluster A's `_sys:nix-cache:narinfo:`
- **THEN** periodic sync and gossip-triggered sync for that subscription SHALL stop
- **AND** the subscription record SHALL be removed from KV
- **AND** previously synced data SHALL remain in the local KV store (not deleted)
