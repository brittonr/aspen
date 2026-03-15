## MODIFIED Requirements

### Requirement: Nix build execution

The CI nix executor SHALL support two build execution modes: in-process via `snix-build`'s `BuildService` (default when `snix-build` feature enabled) and subprocess via `nix build` CLI (fallback when `nix-cli-fallback` feature enabled).

#### Scenario: In-process build (snix-build)

- **WHEN** a CI job of type `ci_nix_build` is dispatched
- **AND** the `snix-build` feature is enabled
- **THEN** the executor SHALL evaluate the flake, convert the derivation to a `BuildRequest`, and execute it via `BuildService::do_build`

#### Scenario: Subprocess build (fallback)

- **WHEN** a CI job of type `ci_nix_build` is dispatched
- **AND** the `snix-build` feature is NOT enabled or initialization fails
- **AND** the `nix-cli-fallback` feature IS enabled
- **THEN** the executor SHALL spawn `nix build` as a subprocess (current behavior)

#### Scenario: Neither mode available

- **WHEN** a CI job of type `ci_nix_build` is dispatched
- **AND** neither `snix-build` nor `nix-cli-fallback` is available
- **THEN** the executor SHALL return a failure with a message indicating no build backend is configured
