## ADDED Requirements

### Requirement: JSON Schema derivation for CI agent protocol

All protocol types in `crates/aspen-ci/src/agent/protocol.rs` — including `HostMessage`, `AgentMessage`, `ExecutionRequest`, `ExecutionResult`, and `LogMessage` — SHALL derive `schemars::JsonSchema`.

#### Scenario: Schema derivation compiles

- **WHEN** the aspen-ci crate is built
- **THEN** all protocol types successfully derive `JsonSchema` alongside existing `Serialize`/`Deserialize`

#### Scenario: Schema generation

- **WHEN** a test calls `schemars::schema_for::<HostMessage>()` and `schemars::schema_for::<AgentMessage>()`
- **THEN** valid JSON Schema documents are produced that describe the protocol types

### Requirement: Nickel contract snapshot test

A test SHALL exist that generates Nickel contracts from the CI agent protocol types and compares them against a checked-in snapshot file at `schemas/ci-agent-protocol.ncl`.

#### Scenario: Snapshot matches

- **WHEN** `cargo nextest run` includes the schema snapshot test and the protocol types have not changed
- **THEN** the test passes

#### Scenario: Snapshot drift detected

- **WHEN** a developer modifies a protocol type (adds a field, changes a variant)
- **THEN** the schema snapshot test fails until the developer updates `schemas/ci-agent-protocol.ncl`

#### Scenario: Snapshot update workflow

- **WHEN** the snapshot test fails due to an intentional protocol change
- **THEN** running the test with `UPDATE_SNAPSHOTS=1` (or equivalent env var) overwrites the snapshot file with the new Nickel contracts

### Requirement: Nickel contract coverage for deploy protocol

The deploy executor types `DeployRequest`, `DeployInitResult`, `DeployStatusResult`, and `DeployNodeStatus` SHALL derive `JsonSchema` with corresponding Nickel contracts at `schemas/deploy-protocol.ncl`.

#### Scenario: Deploy schema generation

- **WHEN** schema generation runs for deploy types
- **THEN** valid Nickel contracts are produced covering all fields of `DeployRequest`, `DeployInitResult`, `DeployStatusResult`, and `DeployNodeStatus`

### Requirement: Schema-to-Nickel converter

A converter module (`schema_gen.rs`) SHALL translate `schemars::schema::RootSchema` to Nickel contract syntax matching the project's existing `ci_schema.ncl` style.

#### Scenario: Converter produces idiomatic Nickel

- **WHEN** a `RootSchema` for a struct with required, optional, and defaulted fields is converted
- **THEN** the output uses `| Type`, `| optional`, and `| default = value` annotations

### Requirement: Schema files checked into repository

All generated schema files under `schemas/` SHALL be checked into version control so protocol changes are visible in code review diffs.

#### Scenario: Schema files in git

- **WHEN** `git ls-files schemas/` is run
- **THEN** it lists `ci-agent-protocol.ncl` and `deploy-protocol.ncl`
