## ADDED Requirements

### Requirement: Per-IP rate limiting for EVENT submissions

The relay SHALL enforce a per-source-IP token-bucket rate limit on EVENT message processing. Each IP address SHALL have an independent bucket with configurable burst capacity and sustained refill rate. Rate limiting SHALL occur before event validation and storage.

#### Scenario: Client within rate limit

- **WHEN** a client submits an EVENT and their IP bucket has remaining tokens
- **THEN** the relay SHALL deduct one token and process the event normally

#### Scenario: Client exceeds rate limit

- **WHEN** a client submits an EVENT and their IP bucket has zero remaining tokens
- **THEN** the relay SHALL respond with `["OK", <event_id>, false, "rate-limited: too many events from this address"]` and NOT validate, store, or broadcast the event

#### Scenario: Token refill over time

- **WHEN** a client's IP bucket was depleted and sufficient time has elapsed for token refill
- **THEN** the relay SHALL allow new EVENT submissions up to the refilled token count

#### Scenario: Burst allowance

- **WHEN** a client has not submitted events recently and sends a rapid burst
- **THEN** the relay SHALL allow up to MAX_EVENTS_BURST_PER_IP (20) events in quick succession before rate limiting applies

### Requirement: Per-pubkey rate limiting for EVENT submissions

The relay SHALL enforce a per-author-pubkey token-bucket rate limit on EVENT message processing. Each pubkey (from the event's `pubkey` field) SHALL have an independent bucket with configurable burst capacity and sustained refill rate. This limit is checked after signature verification to prevent unsigned events from consuming pubkey rate budget.

#### Scenario: Author within rate limit

- **WHEN** a valid EVENT is submitted and the author's pubkey bucket has remaining tokens
- **THEN** the relay SHALL deduct one token and store the event normally

#### Scenario: Author exceeds rate limit

- **WHEN** a valid EVENT is submitted and the author's pubkey bucket has zero remaining tokens
- **THEN** the relay SHALL respond with `["OK", <event_id>, false, "rate-limited: too many events from this author"]` and NOT store or broadcast the event

#### Scenario: Different authors on same IP

- **WHEN** two different pubkeys submit events from the same IP address
- **THEN** each pubkey SHALL have its own independent rate limit bucket, separate from the IP bucket

### Requirement: Rate limit configuration

The relay configuration SHALL include rate limit parameters that control burst capacity and sustained rate for both IP and pubkey dimensions.

#### Scenario: Default rate limits

- **WHEN** no rate limit configuration is specified
- **THEN** the relay SHALL use default values: MAX_EVENTS_PER_SECOND_PER_IP (10), MAX_EVENTS_BURST_PER_IP (20), MAX_EVENTS_PER_SECOND_PER_PUBKEY (5), MAX_EVENTS_BURST_PER_PUBKEY (10)

#### Scenario: Custom rate limits

- **WHEN** the configuration specifies custom rate limit values
- **THEN** the relay SHALL use the configured values instead of defaults

#### Scenario: Rate limiting disabled

- **WHEN** the configuration sets rate limit values to zero
- **THEN** the relay SHALL skip rate limit checks and process all events without throttling

### Requirement: Stale rate limit bucket cleanup

The relay SHALL periodically evict rate limit buckets that have not been accessed within a configurable window to prevent unbounded memory growth.

#### Scenario: Stale bucket evicted

- **WHEN** an IP or pubkey bucket has not been accessed for RATE_LIMIT_BUCKET_TTL_SECS (300) seconds
- **THEN** the relay SHALL remove the bucket from memory on the next cleanup sweep

#### Scenario: Cleanup frequency

- **WHEN** the relay is running
- **THEN** a cleanup sweep SHALL run every RATE_LIMIT_CLEANUP_INTERVAL_SECS (60) seconds
