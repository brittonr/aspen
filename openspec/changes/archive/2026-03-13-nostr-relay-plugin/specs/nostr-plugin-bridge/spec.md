## ADDED Requirements

### Requirement: nostr_publish_event host function

The WASM plugin host SHALL expose a `nostr_publish_event(event_json: &str) -> Result<String, Error>` function. On success, the function SHALL return the event ID. On failure, it SHALL return a descriptive error.

#### Scenario: Plugin publishes valid event

- **WHEN** a WASM plugin calls `nostr_publish_event` with valid NIP-01 event JSON
- **THEN** the host SHALL validate the event, store it in the relay's KV storage, broadcast it to matching subscriptions, and return the event ID

#### Scenario: Plugin publishes invalid event

- **WHEN** a WASM plugin calls `nostr_publish_event` with malformed JSON or an invalid signature
- **THEN** the host SHALL return an error and NOT store or broadcast the event

#### Scenario: Plugin without permission denied

- **WHEN** a WASM plugin with `nostr_publish: false` calls `nostr_publish_event`
- **THEN** the host SHALL return a permission denied error

#### Scenario: Relay not enabled

- **WHEN** a WASM plugin calls `nostr_publish_event` but the `nostr-relay` feature is not active
- **THEN** the host SHALL return an error indicating the Nostr relay is not available

### Requirement: nostr_publish permission in PluginPermissions

The `PluginPermissions` struct SHALL include a `nostr_publish: bool` field defaulting to `false`. The host SHALL check this permission before executing `nostr_publish_event`.

#### Scenario: Permission declared in manifest

- **WHEN** a plugin manifest includes `"permissions": { "nostr_publish": true }`
- **THEN** the plugin SHALL be allowed to call `nostr_publish_event`

#### Scenario: Default permission is denied

- **WHEN** a plugin manifest does not specify `nostr_publish`
- **THEN** the permission SHALL default to `false` and calls to `nostr_publish_event` SHALL be rejected

### Requirement: Event validation at host boundary

The host SHALL validate all events received via `nostr_publish_event` before storing or broadcasting. Validation SHALL include JSON structure conformance, event ID verification (SHA-256 of serialized event data), and secp256k1 Schnorr signature verification.

#### Scenario: Event with incorrect ID rejected

- **WHEN** a plugin publishes an event whose `id` field does not match the SHA-256 of its serialized content
- **THEN** the host SHALL reject the event with an error describing the ID mismatch

#### Scenario: Event with wrong signature rejected

- **WHEN** a plugin publishes an event with a valid structure but an incorrect signature
- **THEN** the host SHALL reject the event with a signature verification error
