## ADDED Requirements

### Requirement: Full eval→build→upload pipeline runs in-memory

An integration test SHALL drive the complete native build pipeline — from `Derivation` construction through `NativeBuildService::build_derivation` to `upload_native_outputs` — using only in-memory snix services and a `FakeBuildService`. No subprocess SHALL be spawned.

#### Scenario: Successful in-memory pipeline

- **WHEN** a test constructs a `Derivation`, resolves inputs against an in-memory `PathInfoService` pre-populated with synthetic entries, and calls `build_derivation`
- **THEN** the `FakeBuildService::do_build` is invoked with a correct `BuildRequest`, returns canned `BuildResult`, and `upload_native_outputs` stores the outputs in the in-memory `PathInfoService`

#### Scenario: PathInfoService contains uploaded outputs after pipeline

- **WHEN** the pipeline completes successfully
- **THEN** each output store path is retrievable via `PathInfoService::get()` with correct `nar_size`, `nar_sha256`, and `references`

### Requirement: NixEvaluator in-memory flake eval

An integration test SHALL evaluate a synthetic flake (no inputs, raw `derivation` builtin) fully in-process via `NixEvaluator::evaluate_flake_derivation` using in-memory snix services. No flake lock generation subprocess SHALL be called.

#### Scenario: Simple flake with empty inputs

- **WHEN** a temp directory contains a `flake.nix` with `inputs = {}` and a matching `flake.lock`, and `evaluate_flake_derivation` is called
- **THEN** returns `Ok((StorePath, Derivation))` with the derivation extracted from `KnownPaths`

#### Scenario: Npins project eval in-memory

- **WHEN** a temp directory contains a `default.nix` returning a raw derivation and `npins/sources.json`, and `evaluate_npins_derivation` is called
- **THEN** returns `Ok((StorePath, Derivation))` with no subprocess spawned

### Requirement: NativeBuildService construction with FakeBuildService

An integration test SHALL construct a `NativeBuildService::with_build_service` using a `FakeBuildService` and in-memory castore services, confirming the wiring compiles and runs.

#### Scenario: with_build_service accepts FakeBuildService

- **WHEN** `NativeBuildService::with_build_service(Box::new(fake), blob_svc, dir_svc, pathinfo_svc)` is called
- **THEN** the resulting service is usable and `build_derivation` routes through the fake

### Requirement: Pipeline handles build failure gracefully

An integration test SHALL verify that when `FakeBuildService::do_build` returns an error, the pipeline propagates it without panicking.

#### Scenario: Build service returns io::Error

- **WHEN** `FakeBuildService` is configured to return `Err(io::Error::other("sandbox failed"))`
- **THEN** `build_derivation` returns `Err` with the error message preserved

### Requirement: Input resolution with pre-populated PathInfoService

An integration test SHALL pre-populate the in-memory `PathInfoService` with synthetic `PathInfo` entries and verify that `resolve_build_inputs` (via `build_derivation`) correctly maps them to `Node` values in the `BuildRequest`.

#### Scenario: Known input resolved from PathInfoService

- **WHEN** a derivation has an `input_sources` entry matching a `PathInfo` in the service
- **THEN** the resolved input appears in the `BuildRequest::inputs` map with the correct `Node`

#### Scenario: Unknown input logged as warning

- **WHEN** a derivation references an input store path not in `PathInfoService` and not on local disk
- **THEN** the input is skipped with a warning (not a hard failure)
