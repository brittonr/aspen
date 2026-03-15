## ADDED Requirements

### Requirement: In-process flake evaluation

The CI executor SHALL support evaluating Nix flake expressions in-process using `snix-eval` and `snix-glue` without spawning the `nix` binary.

#### Scenario: Evaluate flake attribute

- **WHEN** a CI job requests evaluation of `packages.x86_64-linux.default` from a flake
- **THEN** the evaluator SHALL parse and evaluate the Nix expression in-process
- **AND** return the derivation result

#### Scenario: Evaluation error with diagnostics

- **WHEN** a flake contains a Nix syntax or type error
- **THEN** the evaluator SHALL return a structured error with source location, line number, and error message

### Requirement: Store-backed evaluation IO

The evaluator SHALL use `snix-glue`'s `SnixStoreIO` to resolve store paths from Aspen's `BlobService`, `DirectoryService`, and `PathInfoService`.

#### Scenario: Evaluator reads file from store

- **WHEN** a Nix expression reads a file from `/nix/store/...`
- **THEN** the evaluator SHALL resolve the file contents through Aspen's BlobService

#### Scenario: Evaluator resolves flake input

- **WHEN** a flake.lock references a tarball input
- **THEN** the `snix-glue` fetcher SHALL download and unpack it, storing the result in Aspen's castore

### Requirement: Pre-flight flake validation

The CI system SHALL validate that a flake evaluates before queuing a build job.

#### Scenario: Valid flake passes pre-flight

- **WHEN** a CI pipeline receives a push to a repo containing a valid `flake.nix`
- **THEN** the system SHALL evaluate the flake to confirm it produces derivations
- **AND** proceed to queue build jobs

#### Scenario: Invalid flake fails pre-flight

- **WHEN** a CI pipeline receives a push to a repo containing an invalid `flake.nix`
- **THEN** the system SHALL report the evaluation error without queuing any build jobs

### Requirement: Derivation-to-BuildRequest conversion

The evaluator SHALL convert evaluated derivations into `snix-build::BuildRequest` structs for the build service.

#### Scenario: Convert derivation to build request

- **WHEN** evaluation produces a derivation with builder, args, env vars, and input paths
- **THEN** the system SHALL produce a `BuildRequest` with `command_args`, `environment_vars`, `inputs` (as castore Nodes), `outputs`, and `scratch_paths`

### Requirement: Subprocess fallback for unsupported evaluations

When `snix-eval` cannot evaluate an expression (missing builtins, IFD), the system SHALL fall back to `nix eval` subprocess.

#### Scenario: IFD triggers fallback

- **WHEN** evaluation encounters import-from-derivation (IFD)
- **AND** the `nix-cli-fallback` feature is enabled
- **THEN** the system SHALL fall back to subprocess `nix eval` and log a warning
