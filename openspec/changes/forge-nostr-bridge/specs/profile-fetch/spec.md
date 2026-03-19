## ADDED Requirements

### Requirement: Resolve npub to display name

The web UI SHALL resolve an npub to a Nostr display name by querying the relay for kind 0 profile metadata events.

#### Scenario: Profile exists in relay

- **WHEN** the relay has a kind 0 event for an npub containing a display_name field
- **THEN** the web UI SHALL show the display_name instead of the truncated npub

#### Scenario: No profile in relay

- **WHEN** the relay has no kind 0 event for an npub
- **THEN** the web UI SHALL fall back to the truncated npub format

### Requirement: Profile cache

Profile lookups SHALL be cached to avoid repeated queries.

#### Scenario: Same author rendered twice on one page

- **WHEN** the same npub appears in multiple commits
- **THEN** the profile SHALL be fetched once and reused from cache
