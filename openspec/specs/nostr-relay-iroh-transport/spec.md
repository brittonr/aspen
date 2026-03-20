## ADDED Requirements

### Requirement: ALPN registration for Nostr relay

The relay SHALL register a `NOSTR_WS_ALPN` (`b"/aspen/nostr-ws/1"`) protocol handler on the cluster's iroh endpoint when the `nostr-relay` feature is enabled. The protocol handler SHALL accept incoming iroh QUIC connections and route them to the relay's connection handler.

#### Scenario: Iroh client connects to relay

- **WHEN** an iroh peer opens a connection with ALPN `/aspen/nostr-ws/1`
- **THEN** the relay SHALL accept the connection and process Nostr messages on it

#### Scenario: Feature disabled

- **WHEN** the `nostr-relay` feature is not enabled
- **THEN** no NOSTR_WS_ALPN handler SHALL be registered on the iroh endpoint

#### Scenario: Relay not enabled in config

- **WHEN** the `nostr-relay` feature is enabled but `nostr_relay.enabled = false`
- **THEN** the NOSTR_WS_ALPN handler SHALL reject incoming connections

### Requirement: Length-prefixed framing over QUIC streams

Nostr messages exchanged over iroh QUIC streams SHALL use length-prefixed framing: a 4-byte big-endian message length followed by the UTF-8 JSON payload. Each Nostr protocol message (EVENT, REQ, CLOSE, AUTH, OK, EOSE, NOTICE) SHALL be a single framed message.

#### Scenario: Client sends EVENT over iroh

- **WHEN** an iroh client sends a 4-byte length prefix followed by a NIP-01 EVENT JSON payload
- **THEN** the relay SHALL parse the frame, extract the JSON, and process it identically to a WebSocket EVENT message

#### Scenario: Relay sends OK over iroh

- **WHEN** the relay responds to an EVENT submission from an iroh client
- **THEN** the relay SHALL send the `["OK", ...]` response as a length-prefixed frame on the same QUIC stream

#### Scenario: Maximum frame size enforced

- **WHEN** an iroh client sends a frame with a length prefix exceeding MAX_EVENT_SIZE (64 KB)
- **THEN** the relay SHALL close the stream with an error and NOT attempt to read the oversized payload

### Requirement: Iroh transport coexists with TCP WebSocket

The iroh QUIC transport SHALL operate alongside the existing TCP WebSocket listener. Both transports SHALL share the same event store, subscription registry, and broadcast channel.

#### Scenario: Event published via iroh appears on WebSocket

- **WHEN** an iroh client publishes an event via the QUIC transport
- **THEN** the event SHALL be stored and broadcast to WebSocket subscribers with matching filters

#### Scenario: Event published via WebSocket appears on iroh

- **WHEN** a WebSocket client publishes an event
- **THEN** the event SHALL be broadcast to iroh-connected subscribers with matching filters

#### Scenario: Mixed transport subscriptions

- **WHEN** an iroh client and a WebSocket client both subscribe with the same filter
- **AND** a new matching event is published via either transport
- **THEN** both clients SHALL receive the event

### Requirement: ALPN constant in transport crate

The `NOSTR_WS_ALPN` constant SHALL be defined in `crates/aspen-transport/src/constants.rs` alongside other ALPN identifiers.

#### Scenario: Constant available

- **WHEN** code imports `aspen_transport::constants::NOSTR_WS_ALPN`
- **THEN** the value SHALL be `b"/aspen/nostr-ws/1"`
