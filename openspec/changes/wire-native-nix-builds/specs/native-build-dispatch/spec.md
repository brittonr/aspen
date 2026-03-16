## ADDED Requirements

### Requirement: Native build dispatch in execute_build

The `execute_build()` method SHALL attempt native snix-build execution before falling back to subprocess when the `snix-build` feature is enabled and `NativeBuildService` is initialized.

#### Scenario: Native build succeeds

- **WHEN** `snix-build` feature is enabled AND `NativeBuildService` is initialized AND the flake evaluates to a valid `.drv` AND the sandbox builds it successfully
- **THEN** `execute_build()` SHALL return a `NixBuildOutput` produced by `execute_native()` without spawning a `nix build` subprocess

#### Scenario: Native build unavailable — feature off

- **WHEN** `snix-build` feature is NOT enabled
- **THEN** `execute_build()` SHALL use the subprocess path (`spawn_nix_build`) with no change in behavior

#### Scenario: Native build unavailable — service not initialized

- **WHEN** `snix-build` feature is enabled BUT `NativeBuildService` is None (sandbox detection failed at startup)
- **THEN** `execute_build()` SHALL skip the native path and use the subprocess path

#### Scenario: Native build fallback on eval failure

- **WHEN** `resolve_drv_path()` fails (nix eval error, timeout, missing nix binary)
- **THEN** `execute_build()` SHALL log a warning and fall back to the subprocess path

#### Scenario: Native build fallback on build failure

- **WHEN** `resolve_drv_path()` succeeds BUT `execute_native()` returns an error (sandbox crash, missing input, IO error)
- **THEN** `execute_build()` SHALL log a warning and fall back to the subprocess path

### Requirement: Derivation path resolution

The `resolve_drv_path()` method SHALL resolve a `NixBuildPayload` to a `.drv` file path in `/nix/store/` by running `nix eval --raw <flake_ref>.drvPath`.

#### Scenario: Successful resolution

- **WHEN** the flake ref is valid and evaluates to a derivation
- **THEN** `resolve_drv_path()` SHALL return the absolute path to the `.drv` file (e.g., `/nix/store/...-hello.drv`)

#### Scenario: Invalid flake ref

- **WHEN** the flake ref does not evaluate to a derivation (syntax error, missing attribute)
- **THEN** `resolve_drv_path()` SHALL return an error with the nix eval stderr

#### Scenario: Timeout

- **WHEN** `nix eval` does not complete within the configured timeout
- **THEN** `resolve_drv_path()` SHALL kill the subprocess and return a timeout error

### Requirement: Native build output upload

After a successful native build, outputs SHALL be uploaded to the Aspen PathInfoService via `upload_native_outputs()` when SNIX services are configured.

#### Scenario: SNIX services available

- **WHEN** native build succeeds AND `snix_pathinfo_service` is configured
- **THEN** each output SHALL be stored as a `PathInfo` entry with its `Node`, references, and NAR metadata

#### Scenario: SNIX services unavailable

- **WHEN** native build succeeds BUT `snix_pathinfo_service` is None
- **THEN** output paths SHALL still be returned in `NixBuildOutput` but no PathInfo upload occurs

### Requirement: Worker startup initialization

The node binary SHALL call `init_native_build_service()` on the `NixBuildWorker` during startup, before the worker begins accepting jobs.

#### Scenario: Sandbox available

- **WHEN** bubblewrap or OCI sandbox is detected on the system
- **THEN** `NativeBuildService` SHALL be initialized and `has_native_builds()` SHALL return true

#### Scenario: No sandbox available

- **WHEN** neither bubblewrap nor OCI sandbox is available
- **THEN** `NativeBuildService` SHALL remain None AND `has_native_builds()` SHALL return false AND the worker SHALL still accept jobs (via subprocess)
