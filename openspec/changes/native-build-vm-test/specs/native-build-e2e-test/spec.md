## ADDED Requirements

### Requirement: Native build e2e VM test

A NixOS VM integration test SHALL validate the full native snix-build pipeline from flake evaluation through cache gateway serving.

#### Scenario: Cluster boots with native build service

- **WHEN** aspen-node starts with `snix-build` feature enabled and bubblewrap on PATH
- **THEN** the node logs SHALL indicate native build service initialization with bubblewrap backend

#### Scenario: Nix build job completes via native path

- **WHEN** a `ci_nix_build` job is submitted for a trivial flake derivation
- **THEN** the job SHALL complete successfully with output paths in `/nix/store/`

#### Scenario: Build output stored in PathInfoService

- **WHEN** the native build completes
- **THEN** the output store path SHALL be queryable via `nix path-info` style lookup or cache gateway narinfo

#### Scenario: Cache gateway serves natively-built path

- **WHEN** the cache gateway is started and queried for the built store path's narinfo
- **THEN** the gateway SHALL return a valid narinfo response with `StorePath`, `NarHash`, and `NarSize` fields
