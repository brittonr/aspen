# No-Std API Surface Specification

## Purpose

Defines which types MUST be available from `aspen_core` in alloc-only
(no-std) mode and from `aspen-core-shell` without optional features.

## ADDED Requirements

### Requirement: Alloc-safe type availability

The following types MUST be importable from `aspen_core` with
`default-features = false`:

- `AppRegistry` (or alloc-safe subset)
- `NetworkTransport` trait (or abstract version without iroh types)
- `SimulationArtifact` (or alloc-safe data model)
- `SM_KV_TABLE` name constant (or alloc-safe table metadata)

#### Scenario: No-std consumer imports core types

- GIVEN a crate depending on `aspen-core` with `default-features = false`
- WHEN the crate uses `aspen_core::AppRegistry`
- THEN the import resolves and the type is usable

### Requirement: Feature-independent trait availability

The following types MUST be importable from `aspen-core-shell` without
their current feature gates:

- `ContentDiscovery` trait (currently gated behind `global-discovery`)
- `DirectoryLayer` struct (currently gated behind `layer`)

#### Scenario: Shell consumer without global-discovery

- GIVEN a crate depending on `aspen-core-shell` without `global-discovery`
- WHEN the crate uses `aspen_core::ContentDiscovery`
- THEN the import resolves
