## ADDED Requirements

### Requirement: Coupled timeouts derived from base values

Timeout constants that depend on an interval and a miss count SHALL be derived via multiplication rather than defined independently.

#### Scenario: Heartbeat timeout derived from interval

- **WHEN** heartbeat-related constants are defined
- **THEN** the timeout SHALL be computed as `HEARTBEAT_INTERVAL × MAX_MISSED_HEARTBEATS`
- **AND** changing the interval SHALL automatically change the timeout

#### Scenario: No independent coupled constants

- **WHEN** a timeout constant exists that logically depends on another constant
- **THEN** it SHALL be expressed as a `const` expression of its dependencies
- **AND** it SHALL NOT be a separately hardcoded value

### Requirement: Compile-time assertions on derived values

Derived timeout constants SHALL have compile-time assertions validating their relationships.

#### Scenario: Timeout exceeds interval

- **WHEN** a derived timeout is defined
- **THEN** a `const _: () = assert!(TIMEOUT > INTERVAL)` assertion SHALL exist
- **AND** the assertion SHALL fail at compile time if the relationship is violated
