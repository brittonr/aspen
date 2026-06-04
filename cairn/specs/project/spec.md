# project Specification

## Purpose
Capture the current Molten repository baseline before runtime architecture work lands.

## Requirements

### Requirement: Rust and Nix scaffold
r[molten.project.scaffold] The repository MUST provide a Rust workspace with a library crate, a binary crate, and a Nix development shell.

#### Scenario: Developer runs the baseline test
r[molten.project.scaffold.test]
- GIVEN a checkout of the repository
- WHEN a developer runs the configured Rust test command
- THEN the scaffold library test passes

### Requirement: Runtime integration dependencies
r[molten.project.dependencies] The repository MUST declare dependencies for Preserves, Syndicate, Iroh blobs/docs/gossip, Nickel, Steel, Wasmtime/WASI/component tooling, Redb, Blake3, Snafu, Serde, Tracing, Clap, Basalt, Cairn, Octet Valence, and Trellis, plus a Hegel dev dependency for property-based testing.

#### Scenario: Developer inspects project dependencies
r[molten.project.dependencies.inspect]
- GIVEN the repository manifest
- WHEN a developer reviews declared dependencies
- THEN the manifest identifies the crates intended for the Molten runtime integration
