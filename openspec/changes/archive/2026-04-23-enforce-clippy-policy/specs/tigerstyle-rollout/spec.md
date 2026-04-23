## ADDED Requirements

### Requirement: Clippy policy rollout scope is explicit
ID: tigerstyle-rollout.clippy-policy-rollout-scope-is-explicit

The system SHALL roll out enforced Clippy deny policy through an explicit, reviewable crate inventory rather than an implicit workspace-wide assumption.

#### Scenario: Initial enforcement pass names participating crates
ID: tigerstyle-rollout.clippy-policy-rollout-scope-is-explicit.initial-enforcement-pass-names-participating-crates

- **GIVEN** a change that converts Clippy from advisory warnings to executable repository policy
- **WHEN** the first enforcement pass is prepared
- **THEN** the change SHALL list every participating crate root in its rollout inventory
- **AND** it SHALL save the baseline `cargo clippy` command and final `cargo clippy` command for that exact scope
- **AND** reviewers SHALL be able to tell which crates remain outside the first enforcement pass

#### Scenario: Deferred crates stay explicit
ID: tigerstyle-rollout.clippy-policy-rollout-scope-is-explicit.deferred-crates-stay-explicit

- **GIVEN** some workspace crates are not yet in the first enforcement pass
- **WHEN** the rollout inventory is reviewed
- **THEN** each deferred crate SHALL be named explicitly
- **AND** the inventory SHALL record why it is deferred
- **AND** the inventory SHALL name an owner or follow-up change reference for each deferred crate
- **AND** reviewers SHALL be able to distinguish intentional deferral from missing rollout coverage
