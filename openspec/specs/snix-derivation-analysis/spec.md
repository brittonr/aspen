## ADDED Requirements

### Requirement: Parse derivation files

The CI executor SHALL parse `.drv` files using `nix_compat::derivation::Derivation` to extract build metadata.

#### Scenario: Parse valid derivation

- **WHEN** a `.drv` file is read from the store
- **THEN** the parser SHALL extract `outputs`, `input_derivations`, `input_sources`, `builder`, `args`, `environment`, and `system` fields

#### Scenario: Parse invalid derivation

- **WHEN** a `.drv` file contains malformed ATerm syntax
- **THEN** the parser SHALL return a `DerivationError`

### Requirement: Build graph construction from derivations

The CI executor SHALL construct a dependency graph from parsed derivations to determine build order.

#### Scenario: Linear dependency chain

- **WHEN** derivation A depends on derivation B which depends on derivation C
- **THEN** the build graph SHALL order them as C → B → A

#### Scenario: Diamond dependency

- **WHEN** derivation A depends on B and C, and both B and C depend on D
- **THEN** the build graph SHALL build D first, then B and C in parallel, then A

### Requirement: Cache invalidation via input hashing

The CI executor SHALL compute a content hash from a derivation's inputs to determine whether a cached build result is still valid.

#### Scenario: Unchanged inputs hit cache

- **WHEN** a derivation's inputs (sources, dependencies, environment) produce the same content hash as a previous build
- **THEN** the executor SHALL skip the build and return the cached result

#### Scenario: Changed input invalidates cache

- **WHEN** a derivation's environment variable changes compared to a cached build
- **THEN** the executor SHALL invalidate the cache entry and execute a fresh build

### Requirement: Closure computation

The CI executor SHALL compute the runtime closure of a store path by traversing `references` in `PathInfo` entries.

#### Scenario: Compute closure of a build output

- **WHEN** the executor requests the closure of `/nix/store/abc123-hello`
- **AND** that path references `/nix/store/def456-glibc` which references `/nix/store/ghi789-gcc-lib`
- **THEN** the closure SHALL contain all three paths
