## ADDED Requirements

### Requirement: Mutation testing infrastructure

The system SHALL include `cargo-mutants` configuration and nextest integration for running mutation tests against verified crates. Mutation testing SHALL be runnable locally and in CI.

#### Scenario: Run mutation tests on a single crate

- **WHEN** a developer runs `cargo mutants -p aspen-coordination --timeout 60`
- **THEN** the tool SHALL generate mutants for all non-test functions in the crate
- **AND** SHALL report surviving mutants (tests that did not catch the mutation)

#### Scenario: Mutation testing skips test-only code

- **WHEN** mutation testing runs on a crate
- **THEN** it SHALL NOT mutate functions inside `#[cfg(test)]` modules or `tests/` directories

### Requirement: Mutation score baselines

The system SHALL establish mutation score baselines for `aspen-coordination`, `aspen-core`, and `aspen-raft` verified modules. A surviving mutant in a verified function SHALL be treated as a test gap requiring a new test.

#### Scenario: Verified function mutant detected

- **WHEN** `cargo-mutants` generates a mutant in `aspen-coordination/src/verified/lock.rs`
- **AND** the mutant survives (no test fails)
- **THEN** the output SHALL identify the function and line where the mutant survived
- **AND** the mutation score report SHALL flag it as a gap

#### Scenario: Baseline recorded per crate

- **WHEN** mutation testing completes for a crate
- **THEN** the mutation score (killed / total mutants) SHALL be recorded
- **AND** subsequent runs SHALL be comparable against the baseline

### Requirement: CI integration for mutation testing

Mutation testing SHALL run as a scheduled nightly CI job, not on every PR. Results SHALL be available as a report.

#### Scenario: Nightly mutation run

- **WHEN** the nightly CI schedule triggers
- **THEN** `cargo-mutants` SHALL run against the configured crates
- **AND** results SHALL be written to a machine-readable output file

#### Scenario: PR does not block on mutations

- **WHEN** a PR is submitted
- **THEN** mutation testing SHALL NOT run as a blocking check
- **AND** only `cargo nextest run -P quick` SHALL gate the PR
