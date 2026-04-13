## ADDED Requirements

### Requirement: Gossip limiter preserves global budget on per-peer rejection

The gossip message rate limiter SHALL treat per-peer and global accounting as one admission decision. A message denied because the sender exceeded its per-peer limit MUST NOT consume shared global budget.

#### Scenario: Noisy peer cannot starve unrelated peer with rejected traffic

- **WHEN** peer A has already exhausted its per-peer burst budget
- **AND** peer A continues sending additional gossip messages
- **THEN** those rejected messages SHALL be denied as per-peer limited
- **AND** they SHALL NOT reduce the global budget available to peer B

#### Scenario: Accepted traffic still consumes shared global budget

- **WHEN** distinct peers send gossip messages that are accepted by both per-peer and global limits
- **THEN** each accepted message SHALL reduce the shared global budget
- **AND** once the configured global burst is exhausted, later messages SHALL be denied as globally limited

### Requirement: Gossip limiter uses deterministic timestamp-driven transitions

The gossip message rate limiter SHALL derive bucket refill and admission outcomes from deterministic helpers that take explicit prior state and a provided timestamp. Backward timestamps MUST NOT replenish tokens or rewind last-access tracking.

#### Scenario: Injected-time and runtime paths agree on allow and deny decisions

- **WHEN** the same initial limiter state and timestamp sequence are applied through deterministic tests and the runtime wrapper
- **THEN** both paths SHALL produce the same allow-or-deny results
- **AND** both paths SHALL leave equivalent bucket state after each step

#### Scenario: Backward timestamps do not rewind limiter state

- **WHEN** a bucket or peer entry has already advanced to a later timestamp
- **AND** a subsequent check is evaluated with an earlier timestamp
- **THEN** the limiter SHALL preserve the later timestamp as its effective last-update or last-access time
- **AND** it SHALL NOT mint extra tokens from the backward jump

### Requirement: Gossip limiter state is module-owned

The gossip rate limiter SHALL expose checked constructor and admission APIs without exposing mutable per-peer entry fields needed to maintain limiter invariants.

#### Scenario: Discovery lifecycle uses limiter through public API only

- **WHEN** gossip discovery integrates `GossipRateLimiter`
- **THEN** it SHALL construct and query the limiter through public limiter methods
- **AND** it SHALL NOT require direct mutation of per-peer token bucket fields or LRU timestamps
