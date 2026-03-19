## ADDED Requirements

### Requirement: Relay sends AUTH challenge on WebSocket connect

The relay SHALL send an `["AUTH", <challenge>]` message to each client immediately after the WebSocket handshake completes. The challenge SHALL be a 64-character hex-encoded string derived from 32 random bytes. Each connection SHALL receive a unique challenge.

#### Scenario: Client connects and receives challenge

- **WHEN** a Nostr client opens a WebSocket connection to the relay
- **THEN** the relay SHALL send `["AUTH", <challenge>]` where challenge is a 64-character hex string

#### Scenario: Each connection gets a unique challenge

- **WHEN** two clients connect to the relay simultaneously
- **THEN** each client SHALL receive a different challenge string

### Requirement: Client authenticates with kind 22242 signed event

The relay SHALL accept `ClientMessage::Auth` containing a kind 22242 event as proof of identity. The relay SHALL verify the event satisfies all NIP-42 requirements before marking the connection as authenticated.

#### Scenario: Valid NIP-42 auth event

- **WHEN** a client sends `["AUTH", <signed_kind_22242_event>]` with correct challenge tag, valid relay tag, valid signature, and created_at within ±60 seconds of current time
- **THEN** the relay SHALL mark the connection as authenticated for the event's pubkey and respond with `["OK", <event_id>, true, ""]`

#### Scenario: Auth event with wrong challenge

- **WHEN** a client sends a kind 22242 event with a challenge tag that does not match the relay's issued challenge
- **THEN** the relay SHALL respond with `["OK", <event_id>, false, "auth-required: invalid challenge"]` and the connection SHALL remain unauthenticated

#### Scenario: Auth event with wrong relay URL

- **WHEN** the relay has a configured `relay_url` AND a client sends a kind 22242 event with a relay tag that does not match the configured URL
- **THEN** the relay SHALL respond with `["OK", <event_id>, false, "auth-required: invalid relay url"]` and the connection SHALL remain unauthenticated

#### Scenario: Auth event with no relay URL configured

- **WHEN** the relay does not have a `relay_url` configured AND a client sends a kind 22242 event without a matching relay tag
- **THEN** the relay SHALL skip the relay URL check and proceed with other validations

#### Scenario: Auth event with expired timestamp

- **WHEN** a client sends a kind 22242 event with `created_at` more than 60 seconds from the relay's current time
- **THEN** the relay SHALL respond with `["OK", <event_id>, false, "auth-required: event too old or too new"]` and the connection SHALL remain unauthenticated

#### Scenario: Auth event with invalid signature

- **WHEN** a client sends a kind 22242 event with an invalid Schnorr signature
- **THEN** the relay SHALL respond with `["OK", <event_id>, false, "auth-required: invalid signature"]` and the connection SHALL remain unauthenticated

#### Scenario: Auth event with wrong kind

- **WHEN** a client sends an AUTH message containing an event that is not kind 22242
- **THEN** the relay SHALL respond with `["OK", <event_id>, false, "auth-required: must be kind 22242"]` and the connection SHALL remain unauthenticated

### Requirement: Write policy controls EVENT acceptance

The relay SHALL enforce a configurable write policy that determines whether unauthenticated clients can submit events via the WebSocket connection. The policy SHALL NOT affect reads (REQ/CLOSE) or internal writes via `publish()`.

#### Scenario: Open policy allows unauthenticated writes

- **WHEN** write policy is `open` AND an unauthenticated client sends an EVENT
- **THEN** the relay SHALL accept the event (if signature is valid) as it does today

#### Scenario: AuthRequired policy rejects unauthenticated writes

- **WHEN** write policy is `auth_required` AND an unauthenticated client sends an EVENT
- **THEN** the relay SHALL respond with `["OK", <event_id>, false, "auth-required: please authenticate"]` and NOT store the event

#### Scenario: AuthRequired policy accepts authenticated writes

- **WHEN** write policy is `auth_required` AND an authenticated client sends an EVENT with a valid signature
- **THEN** the relay SHALL accept and store the event

#### Scenario: ReadOnly policy rejects all external writes

- **WHEN** write policy is `read_only` AND any client (authenticated or not) sends an EVENT
- **THEN** the relay SHALL respond with `["OK", <event_id>, false, "blocked: relay is read-only"]` and NOT store the event

#### Scenario: ReadOnly policy allows internal publish

- **WHEN** write policy is `read_only` AND the bridge or a plugin calls `NostrRelayService::publish()`
- **THEN** the event SHALL be stored and broadcast to subscribers

#### Scenario: Read operations unaffected by write policy

- **WHEN** write policy is `auth_required` or `read_only` AND an unauthenticated client sends a REQ
- **THEN** the relay SHALL process the subscription and return matching events normally

### Requirement: Auth state is per-connection and ephemeral

Auth state SHALL be scoped to the individual WebSocket connection. It SHALL NOT persist across reconnections or be shared between connections.

#### Scenario: Reconnecting client must re-authenticate

- **WHEN** an authenticated client disconnects and reconnects
- **THEN** the new connection SHALL start unauthenticated and the relay SHALL send a new AUTH challenge

#### Scenario: Auth on one connection does not affect others

- **WHEN** a client authenticates on connection A but also has connection B open
- **THEN** connection B SHALL remain unauthenticated unless it independently completes NIP-42
