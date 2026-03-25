## ADDED Requirements

### Requirement: Flake eval via embedded flake-compat

The system SHALL evaluate flake projects by importing NixOS/flake-compat's `default.nix` through snix-eval, using the expression `(import <flake-compat> { src = <flake_dir>; }).outputs.<attr>.drvPath`. Input fetching SHALL be handled by snix-eval's native `fetchTarball` and `builtins.path` builtins.

#### Scenario: Simple flake with no inputs evaluates via flake-compat

- **WHEN** `evaluate_flake_via_compat` is called for a flake with `inputs = {}`
- **THEN** snix-eval evaluates the flake-compat expression successfully
- **AND** a Derivation is extracted from KnownPaths
- **AND** no subprocess is spawned

#### Scenario: Flake with tarball input evaluates via flake-compat

- **WHEN** a flake has a `tarball` input with a locked URL and narHash in flake.lock
- **THEN** snix-eval's `fetchTarball` builtin downloads and unpacks the tarball
- **AND** the narHash is verified against the flake.lock value
- **AND** `import (outPath + "/flake.nix")` reads from the fetched content via SnixStoreIO
- **AND** the flake's outputs function is called with the resolved input

#### Scenario: Flake with github input evaluates via flake-compat

- **WHEN** a flake has a `github` input with owner/repo/rev in flake.lock
- **THEN** flake-compat constructs a GitHub API tarball URL
- **AND** snix-eval's `fetchTarball` downloads the archive
- **AND** evaluation succeeds without subprocess

### Requirement: Flake-compat embedded as pinned resource

The system SHALL embed flake-compat's `default.nix` as a compile-time string constant from a pinned commit. The file SHALL be written to a temp path before evaluation.

#### Scenario: Embedded flake-compat is valid Nix

- **WHEN** the embedded flake-compat string is parsed by snix-eval
- **THEN** no syntax errors are reported

### Requirement: Fallback to subprocess on unsupported input types

When a flake's locked inputs include a `git` type (or any type where snix-eval returns `NotImplemented`), the system SHALL fall back to the `nix eval --raw .drvPath` subprocess path. The fallback SHALL be logged at WARN level with the specific unsupported input type.

#### Scenario: Git input triggers subprocess fallback

- **WHEN** a flake has an input of type `git` in flake.lock
- **AND** snix-eval's evaluation fails with `NotImplemented("fetchGit")`
- **THEN** `try_flake_eval_native` returns an error
- **AND** `try_native_build` falls back to `resolve_drv_and_parse` (nix eval subprocess)
- **AND** a warning log includes "fetchGit" and "falling back"

### Requirement: Zero-subprocess flake build end-to-end

A flake with fetchable inputs (github/gitlab/tarball/path/sourcehut) SHALL be buildable through the fully in-process path: flake-compat eval via snix-eval → Derivation from KnownPaths → build via LocalStoreBuildService → output upload to PathInfoService. No `nix` subprocess SHALL be spawned.

#### Scenario: VM integration test confirms zero-subprocess pipeline

- **WHEN** a ci_nix_build job is submitted for a flake with a tarball input served over local HTTP
- **THEN** the job completes successfully
- **AND** the aspen-node journal contains "zero subprocesses" log line
- **AND** the journal does NOT contain "nix eval --raw" subprocess invocation
- **AND** output paths are uploaded to PathInfoService
- **AND** the cache gateway serves valid narinfo for the output

### Requirement: Existing manual eval path retained as fallback

The current `evaluate_flake_derivation` method (call-flake.rs + flake_lock.rs + fetch.rs) SHALL be retained behind a cfg feature flag as a fallback. The flake-compat path SHALL be tried first.

#### Scenario: Manual eval path available when flake-compat fails

- **WHEN** flake-compat evaluation fails for any reason
- **AND** the legacy eval feature flag is enabled
- **THEN** the system falls back to `evaluate_flake_derivation` (manual path)
- **AND** a warning is logged indicating the flake-compat failure reason
