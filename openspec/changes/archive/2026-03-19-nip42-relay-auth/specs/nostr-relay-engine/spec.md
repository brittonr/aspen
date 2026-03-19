## MODIFIED Requirements

### Requirement: NIP-11 relay information document

The relay SHALL serve a JSON relay information document at the WebSocket endpoint when accessed via HTTP GET with `Accept: application/nostr+json`.

#### Scenario: Relay info requested

- **WHEN** a client sends an HTTP GET with header `Accept: application/nostr+json`
- **THEN** the relay SHALL respond with a JSON document containing `name`, `description`, `pubkey` (the cluster's Nostr npub), `supported_nips` ([1, 11, 34, 42]), and `software` fields

#### Scenario: Auth required advertised in relay info

- **WHEN** write policy is `auth_required` AND a client requests relay info
- **THEN** the relay info document SHALL include `"limitation": { "auth_required": true }` per NIP-11
