## ADDED Requirements

### Requirement: Resource policy unit test coverage

The `resource_policy.rs` module SHALL have tests covering boundary conditions for resource access decisions.

#### Scenario: Empty policy allows nothing

- **WHEN** a resource policy has no rules defined
- **THEN** all access checks SHALL deny

#### Scenario: Wildcard policy allows everything

- **WHEN** a resource policy has a wildcard allow rule
- **THEN** access checks for any resource SHALL permit

#### Scenario: Specific resource deny overrides allow

- **WHEN** a policy has both an allow-all and a deny for a specific resource
- **THEN** access to the denied resource SHALL be rejected

### Requirement: Verification module unit test coverage

The `verification.rs` module SHALL have tests covering content hash verification, signature validation, and error paths.

#### Scenario: Valid content hash passes

- **WHEN** content is verified against its correct BLAKE3 hash
- **THEN** verification SHALL succeed

#### Scenario: Tampered content fails hash check

- **WHEN** content is modified after hashing
- **THEN** verification SHALL fail with a hash mismatch error

#### Scenario: Expired credential rejected

- **WHEN** a credential's expiry timestamp is in the past
- **THEN** verification SHALL reject it

#### Scenario: Missing signature on signed-required resource

- **WHEN** a resource requires delegate signatures and none is provided
- **THEN** verification SHALL reject the content

### Requirement: Selection module unit test coverage

The `selection.rs` module SHALL have tests covering seeder selection, priority ordering, and edge cases.

#### Scenario: No available seeders returns empty

- **WHEN** selection is asked to pick seeders from an empty list
- **THEN** the result SHALL be an empty selection

#### Scenario: Single seeder is always selected

- **WHEN** only one seeder is available
- **THEN** that seeder SHALL be selected

#### Scenario: Untrusted seeders excluded

- **WHEN** some seeders are marked untrusted
- **THEN** untrusted seeders SHALL not appear in the selection

#### Scenario: Selection respects max count

- **WHEN** more seeders are available than the requested max
- **THEN** the selection SHALL contain at most max entries
