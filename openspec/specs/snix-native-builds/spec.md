# snix-native-builds Specification

## Purpose

Defines the Snix Native Builds capability requirements preserved by Aspen's archived OpenSpec records, including subprocess fallback, native build is the default execution path, cache operations use PathInfoService.

## Requirements

### Requirement: Subprocess fallback

The executor SHALL retain the `nix build` subprocess execution path as a runtime fallback. The `nix-cli-fallback` feature flag is removed — the subprocess path is always compiled in when `snix-build` is enabled, and activates only when the native build service fails or is unavailable at runtime.

#### Scenario: Fallback to nix CLI

- **WHEN** `snix-build` feature is enabled
- **AND** the native `BuildService` fails to initialize or a build fails
- **THEN** the executor SHALL fall back to spawning `nix build` as a subprocess
- **AND** the executor SHALL log a warning indicating fallback was triggered


### Requirement: Native build is the default execution path

The native snix-build pipeline SHALL be the default build execution path. All `nix` CLI subprocess calls (`nix eval`, `nix build`, `nix-store --realise`, `nix flake lock`) SHALL be gated behind the `nix-cli-fallback` feature flag.

#### Scenario: Build without nix CLI installed

- **WHEN** the `nix-cli-fallback` feature is disabled and no `nix` binary exists in PATH
- **THEN** the native build path (snix-eval → bwrap sandbox → PathInfoService upload) executes without error for derivations whose inputs are resolvable from PathInfoService or upstream caches

#### Scenario: Build with nix-cli-fallback enabled

- **WHEN** the `nix-cli-fallback` feature is enabled and the native path fails (eval error, missing inputs)
- **THEN** the system falls back to `nix eval`, `nix-store --realise`, or `nix build` subprocesses as last resort

#### Scenario: Feature flag gating

- **WHEN** code calls `resolve_drv_path`, `spawn_nix_build`, `ensure_flake_lock`, or `nix-store --realise`
- **THEN** the call site is inside a `#[cfg(feature = "nix-cli-fallback")]` block

### Requirement: Cache operations use PathInfoService

The `cache.rs` module SHALL use PathInfoService for store path metadata queries and snix-store's SimpleRenderer for NAR serialization, instead of `nix path-info --json` and `nix nar dump-path` subprocesses.

#### Scenario: Store path size check via PathInfoService

- **WHEN** `check_store_path_size` is called with snix services configured
- **THEN** the NAR size is retrieved from PathInfoService without subprocess

#### Scenario: NAR upload via castore ingestion

- **WHEN** `upload_store_paths` is called with snix services configured
- **THEN** the store path is ingested into BlobService/DirectoryService and registered in PathInfoService without `nix nar dump-path`

#### Scenario: Fallback when snix services unavailable

- **WHEN** snix services are not configured (e.g., snix feature disabled)
- **THEN** the system falls back to subprocess calls if `nix-cli-fallback` is enabled, or returns an error if not
