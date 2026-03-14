## ADDED Requirements

### Requirement: flake-parts module system

The project SHALL use `flake-parts` as its flake composition framework, replacing direct `flake-utils` usage in the root `flake.nix`.

#### Scenario: Root flake structure

- **WHEN** `flake.nix` is inspected
- **THEN** it imports `flake-parts` and delegates to modules via the `imports` list
- **THEN** the root file contains only input declarations and module imports, not build logic

#### Scenario: flake-parts input

- **WHEN** `flake.nix` inputs are inspected
- **THEN** `flake-parts` is listed as an input and `flake-utils` is removed

### Requirement: Per-concern module files

Build logic SHALL be split into separate module files under `nix/flake-modules/`, with each file responsible for a single concern.

#### Scenario: Module file organization

- **WHEN** `nix/flake-modules/` is listed
- **THEN** it contains at minimum separate files for: Rust builds, checks/lints, NixOS VM tests, dev shell, dogfood pipeline, Verus verification, VM images, and apps

#### Scenario: Module isolation

- **WHEN** a single module file is edited
- **THEN** only the outputs related to that module's concern are affected

### Requirement: Build output equivalence

The migration to `flake-parts` SHALL NOT change any build outputs. All packages, checks, dev shells, apps, and NixOS VM tests SHALL produce identical results before and after the migration.

#### Scenario: Package output parity

- **WHEN** `nix build .#<package>` is run after migration
- **THEN** the output store path matches the pre-migration build for the same inputs

#### Scenario: Check output parity

- **WHEN** `nix flake check` is run after migration
- **THEN** all checks that passed before migration continue to pass

### Requirement: Incremental migration path

The migration SHALL be performed incrementally — one module extracted at a time — with `nix flake check` passing after each extraction step.

#### Scenario: Single module extraction

- **WHEN** one concern is extracted from `flake.nix` into a module file
- **THEN** `nix flake check` passes before the next extraction begins
