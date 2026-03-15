## ADDED Requirements

### Requirement: Deserialize Nix expressions to Rust config structs

The system SHALL support deserializing Nix language expressions into Rust types using `snix_serde::from_str`.

#### Scenario: Parse simple config

- **WHEN** a Nix expression `{ nodes = 3; replication = 2; timeout_ms = 5000; }` is deserialized into a `ClusterConfig` struct
- **THEN** the result SHALL have `nodes = 3`, `replication = 2`, `timeout_ms = 5000`

#### Scenario: Parse nested config

- **WHEN** a Nix expression with nested attribute sets is deserialized
- **THEN** nested structs SHALL be populated correctly

#### Scenario: Type mismatch error

- **WHEN** a Nix expression produces a string where an integer is expected
- **THEN** deserialization SHALL return a typed error with the field name and expected type

### Requirement: Nix-based CI pipeline definitions

The CI system SHALL accept pipeline definitions written in Nix and deserialized via `snix-serde`.

#### Scenario: Parse pipeline from Nix file

- **WHEN** a repository contains a `.aspen/ci.nix` file defining build stages
- **THEN** the CI system SHALL evaluate and deserialize it into pipeline configuration structs

#### Scenario: Nix pipeline with conditional stages

- **WHEN** a pipeline definition uses Nix `if` expressions to conditionally include stages
- **THEN** the evaluator SHALL resolve the conditionals and produce the final pipeline definition

### Requirement: Pure evaluation only

Config deserialization SHALL use pure Nix evaluation (no I/O, no impure builtins) to prevent side effects during config parsing.

#### Scenario: Impure builtin rejected

- **WHEN** a config Nix expression calls `builtins.readFile` or other I/O builtins
- **THEN** evaluation SHALL fail with an error indicating impure operations are not allowed
