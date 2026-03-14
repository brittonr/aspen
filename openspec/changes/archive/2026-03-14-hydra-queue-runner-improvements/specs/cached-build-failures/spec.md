## ADDED Requirements

### Requirement: Failed derivation paths are cached

When a nix build fails with a deterministic error, the system SHALL store the failed output paths in the KV store to prevent identical rebuild attempts.

#### Scenario: Cache a build failure

- **WHEN** a nix build job fails
- **AND** the failure is not marked as transient/retryable
- **THEN** each output path from the derivation SHALL be stored at `_ci:failed-paths:{blake3_hash}` in the KV store
- **AND** the entry SHALL include a TTL (default: 24 hours)

#### Scenario: Transient failure is not cached

- **WHEN** a nix build job fails with `is_retryable: true`
- **THEN** no failure cache entry SHALL be created for that derivation

### Requirement: Cached failures are checked before dispatch

Before scheduling a nix build job, the executor SHALL check whether the derivation's output paths exist in the failure cache. A cache hit SHALL skip the build immediately.

#### Scenario: Derivation found in failure cache

- **WHEN** a nix build job is about to be dispatched
- **AND** any of its output paths exist in `_ci:failed-paths:`
- **THEN** the job SHALL be marked as `CachedFailure` without scheduling to a worker
- **AND** all dependent jobs SHALL receive `DepFailed` status

#### Scenario: Derivation not in failure cache

- **WHEN** a nix build job is about to be dispatched
- **AND** none of its output paths exist in `_ci:failed-paths:`
- **THEN** the job SHALL proceed to normal scheduling

#### Scenario: Failure cache entry expires

- **WHEN** a failure cache entry's TTL expires
- **THEN** the key SHALL be removed from the KV store
- **AND** subsequent builds of that derivation SHALL proceed normally

### Requirement: Manual retry clears failure cache

When an operator manually retries a failed build, the system SHALL clear any failure cache entries for that derivation before rescheduling.

#### Scenario: Retry clears cache entry

- **WHEN** an operator triggers a manual retry for a failed nix build job
- **THEN** the system SHALL delete `_ci:failed-paths:{blake3_hash}` for each output path of the derivation
- **AND** the job SHALL be rescheduled to a worker

#### Scenario: Retry of non-cached failure

- **WHEN** an operator retries a job that has no failure cache entries
- **THEN** the retry SHALL proceed without error
