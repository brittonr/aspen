## ADDED Requirements

### Requirement: WebSocket listener accepts Nostr client connections

The relay engine SHALL listen on a configurable TCP port (default 4869) for incoming WebSocket connections. The listener SHALL be feature-gated behind `nostr-relay` and disabled by default.

#### Scenario: Client connects to relay

- **WHEN** a Nostr client opens a WebSocket connection to the configured port
- **THEN** the relay SHALL accept the connection and begin processing NIP-01 messages

#### Scenario: Relay disabled by default

- **WHEN** the `nostr-relay` feature is not enabled or `[nostr_relay] enabled = false`
- **THEN** no TCP listener SHALL be started and no resources SHALL be allocated

#### Scenario: Connection limit enforced

- **WHEN** the number of active WebSocket connections reaches MAX_NOSTR_CONNECTIONS (256)
- **THEN** new connection attempts SHALL be rejected with a WebSocket close frame

### Requirement: NIP-01 EVENT message handling

The relay engine SHALL accept `["EVENT", <event JSON>]` messages from clients. The relay SHALL validate the event signature, store the event, and respond with an `["OK", <event_id>, true|false, <message>]` message.

#### Scenario: Valid event published

- **WHEN** a client sends a valid NIP-01 EVENT message with a correct signature
- **THEN** the relay SHALL store the event in the KV store, broadcast it to matching subscriptions, and respond with `["OK", <event_id>, true, ""]`

#### Scenario: Invalid signature rejected

- **WHEN** a client sends an EVENT message with an invalid secp256k1 Schnorr signature
- **THEN** the relay SHALL respond with `["OK", <event_id>, false, "invalid: bad signature"]` and NOT store the event

#### Scenario: Oversized event rejected

- **WHEN** a client sends an EVENT message exceeding MAX_EVENT_SIZE (64 KB)
- **THEN** the relay SHALL respond with `["OK", <event_id>, false, "invalid: event too large"]`

### Requirement: NIP-01 REQ subscription handling

The relay engine SHALL accept `["REQ", <subscription_id>, <filter1>, ...]` messages. The relay SHALL query stored events matching the filters, send them as EVENT messages, send an EOSE marker, and then push new matching events in real-time until the subscription is closed.

#### Scenario: Subscribe and receive stored events

- **WHEN** a client sends a REQ with filters matching 3 stored events
- **THEN** the relay SHALL send 3 `["EVENT", <sub_id>, <event>]` messages followed by `["EOSE", <sub_id>]`

#### Scenario: Real-time event push

- **WHEN** a client has an active subscription and a new event matching its filters is published
- **THEN** the relay SHALL send `["EVENT", <sub_id>, <event>]` to the subscribing client

#### Scenario: Subscription limit enforced

- **WHEN** a client attempts to create more than MAX_SUBSCRIPTIONS_PER_CONNECTION (16) subscriptions
- **THEN** the relay SHALL respond with `["CLOSED", <sub_id>, "error: too many subscriptions"]`

#### Scenario: Subscription replacement

- **WHEN** a client sends a REQ with a subscription_id that already exists on that connection
- **THEN** the relay SHALL replace the old subscription with the new one

### Requirement: NIP-01 CLOSE message handling

The relay engine SHALL accept `["CLOSE", <subscription_id>]` messages and stop sending events for that subscription.

#### Scenario: Close active subscription

- **WHEN** a client sends CLOSE for an active subscription_id
- **THEN** the relay SHALL stop sending events for that subscription and release associated resources

### Requirement: NIP-01 filter matching

The relay engine SHALL support all NIP-01 filter attributes: `ids`, `authors`, `kinds`, `#<tag>`, `since`, `until`, and `limit`. Multiple conditions within a filter SHALL be AND-joined. Multiple filters in a REQ SHALL be OR-joined. Time-range filters (`since`, `until`) SHALL be used to bound KV index scans rather than loading all candidates and filtering in-memory.

#### Scenario: Filter by kind

- **WHEN** a subscription filter specifies `{"kinds": [30617, 30618]}`
- **THEN** only events with kind 30617 or 30618 SHALL match

#### Scenario: Filter by author and kind

- **WHEN** a subscription filter specifies `{"kinds": [1617], "authors": ["<pubkey>"]}`
- **THEN** only events with kind 1617 AND the specified author SHALL match

#### Scenario: Filter by tag

- **WHEN** a subscription filter specifies `{"#a": ["30617:<pubkey>:<repo-id>"]}`
- **THEN** only events with an `a` tag matching the value SHALL match

#### Scenario: Time range filter

- **WHEN** a subscription filter specifies `{"since": 1700000000, "until": 1700100000}`
- **THEN** only events with `created_at` between 1700000000 and 1700100000 inclusive SHALL match

#### Scenario: Time range bounds KV scan

- **WHEN** a subscription filter specifies `since` and/or `until` alongside a `kinds` filter
- **THEN** the KV scan prefix SHALL include the timestamp bound so that entries outside the range are never read from the store

#### Scenario: Limit applied to initial query

- **WHEN** a subscription filter specifies `{"limit": 10}` and 50 events match
- **THEN** the relay SHALL return at most 10 events ordered by `created_at` descending

### Requirement: Event storage in KV

The relay engine SHALL store Nostr events in the Raft-backed KV store with secondary indexes for kind, author, tag, and creation time. The total number of stored events SHALL NOT exceed MAX_STORED_EVENTS (100,000). The stored event counter SHALL be updated atomically using compare-and-swap to prevent count drift under concurrent writes.

#### Scenario: Event persisted and queryable

- **WHEN** a valid event is stored
- **THEN** the event SHALL be retrievable by its event ID and discoverable via kind, author, and tag index scans

#### Scenario: Storage limit enforced

- **WHEN** the stored event count reaches MAX_STORED_EVENTS
- **THEN** the relay SHALL evict the oldest events (by `created_at`) to make room for new events

#### Scenario: Replaceable event handling

- **WHEN** a replaceable event (kind 0, 3, or 10000-19999) is stored with the same pubkey and kind as an existing event
- **THEN** the relay SHALL keep only the event with the latest `created_at`, deleting the older one

#### Scenario: Concurrent event count accuracy

- **WHEN** multiple events are stored concurrently by different connections
- **THEN** the event count SHALL reflect the exact number of stored events, with no drift from lost increments or decrements

#### Scenario: CAS retry exhaustion

- **WHEN** the compare-and-swap retry loop exceeds MAX_CAS_RETRIES (5) attempts
- **THEN** the relay SHALL return a storage error and NOT store the event, rather than proceeding with a potentially inaccurate count

### Requirement: NIP-11 relay information document

The relay SHALL serve a JSON relay information document at the WebSocket endpoint when accessed via HTTP GET with `Accept: application/nostr+json`. The relay SHALL inspect incoming TCP connections for HTTP request headers before attempting a WebSocket upgrade.

#### Scenario: Relay info requested

- **WHEN** a client sends an HTTP GET with header `Accept: application/nostr+json`
- **THEN** the relay SHALL respond with a JSON document containing `name`, `description`, `pubkey` (the cluster's Nostr npub), `supported_nips` ([1, 11, 34, 42]), and `software` fields, and SHALL NOT attempt a WebSocket upgrade

#### Scenario: Auth required advertised in relay info

- **WHEN** write policy is `auth_required` AND a client requests relay info
- **THEN** the relay info document SHALL include `"limitation": { "auth_required": true }` per NIP-11

#### Scenario: WebSocket upgrade proceeds for normal clients

- **WHEN** a client sends an HTTP GET with `Upgrade: websocket` header
- **THEN** the relay SHALL complete the WebSocket handshake and begin NIP-01 message processing

#### Scenario: Non-WebSocket non-NIP-11 request rejected

- **WHEN** a client sends an HTTP request without `Accept: application/nostr+json` and without `Upgrade: websocket`
- **THEN** the relay SHALL respond with HTTP 400 and close the connection

### Requirement: Tiger Style resource bounds

All relay operations SHALL enforce fixed resource limits defined as constants.

#### Scenario: Bounds enforced

- **WHEN** any resource limit is reached (connections, subscriptions, events, message size)
- **THEN** the relay SHALL reject the operation with a descriptive error message rather than crash or allocate unbounded resources
