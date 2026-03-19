## ADDED Requirements

### Requirement: Author display resolves to Nostr profile

The web UI SHALL display Nostr profile information (display name, NIP-05 identifier) for commit authors, issue creators, and comment authors instead of raw hex keys.

#### Scenario: Author has a linked npub with kind 0 profile

- **WHEN** the web UI renders an author whose ed25519 key maps to an npub with a kind 0 profile event in the relay
- **THEN** the display SHALL show the profile's display_name (or name) and NIP-05 identifier

#### Scenario: Author has a linked npub but no profile

- **WHEN** the web UI renders an author whose ed25519 key maps to an npub with no kind 0 event
- **THEN** the display SHALL show the npub in bech32 format (npub1...)

#### Scenario: Author has no linked npub

- **WHEN** the web UI renders an author whose ed25519 key has no npub mapping
- **THEN** the display SHALL show the truncated ed25519 hex key as it does today

### Requirement: Profile cache

Profile lookups SHALL be cached in-memory with a bounded TTL to avoid repeated KV and relay queries.

#### Scenario: Repeated renders of the same author

- **WHEN** the same ed25519 key appears in multiple commits on one page
- **THEN** the profile lookup SHALL happen once and the cached result SHALL be reused

#### Scenario: Cache expiry

- **WHEN** a cached profile entry is older than 5 minutes
- **THEN** the next render SHALL re-fetch the profile from the relay

### Requirement: Author field carries npub

The Author struct in commit objects and COB changes SHALL include an optional npub field so the mapping is embedded in the signed data.

#### Scenario: Commit created by authenticated user

- **WHEN** an authenticated user creates a commit
- **THEN** the Author.npub field SHALL contain their npub hex string

#### Scenario: Commit created without authentication

- **WHEN** an unauthenticated operation creates a commit (node default key)
- **THEN** the Author.npub field SHALL be None
