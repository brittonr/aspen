## ADDED Requirements

### Requirement: CI checkout directories SHALL be valid git repositories

After file extraction and build preparation, the checkout directory SHALL be initialized as a git repository with all extracted files committed. This enables Nix flake evaluation which requires git context.

#### Scenario: Nix flake build in checkout directory

- **WHEN** a CI job with `executor: "nix"` runs in a checkout directory
- **THEN** the directory SHALL contain a `.git/` directory with at least one commit
- **AND** `nix build .#<attribute>` SHALL be able to evaluate the flake

#### Scenario: Commit message contains source traceability

- **WHEN** the git repository is initialized in the checkout directory
- **THEN** the commit message SHALL contain the original Forge commit hash (hex-encoded)
- **AND** the author SHALL be `Aspen CI <ci@aspen>`

### Requirement: Git initialization SHALL be bounded

The git initialization step SHALL complete within the existing checkout resource bounds (500MB, 50K files). The system SHALL NOT fetch external data during git init.

#### Scenario: Large checkout git initialization

- **WHEN** a checkout directory contains 50,000 files totaling 400MB
- **THEN** `git init` + `git add` + `git commit` SHALL complete without error
- **AND** the operation SHALL NOT exceed a 60-second timeout

### Requirement: Missing git binary SHALL produce actionable error

If `git` is not available in the CI execution environment, the system SHALL report a clear error rather than an opaque failure.

#### Scenario: Git not installed

- **WHEN** the `git` binary is not found in `$PATH` during checkout preparation
- **THEN** the system SHALL return an error with message indicating git is required for Nix flake builds
