## ADDED Requirements

### Requirement: Fuzz targets for snix parsers

Fuzz targets SHALL exist for each snix-facing parser that processes potentially untrusted input.

#### Scenario: NAR ingestion fuzz target

- **WHEN** the fuzz suite is run
- **THEN** a target SHALL exercise `snix_store::nar::ingest_nar_and_hash` with arbitrary byte input
- **AND** the target SHALL not panic on any input

#### Scenario: Directory protobuf fuzz target

- **WHEN** the fuzz suite is run
- **THEN** a target SHALL exercise protobuf deserialization of `snix_castore::proto::Directory` messages with arbitrary bytes
- **AND** valid deserialized directories SHALL roundtrip through serialize/deserialize

#### Scenario: PathInfo encoding fuzz target

- **WHEN** the fuzz suite is run
- **THEN** a target SHALL exercise PathInfo protobuf deserialization with arbitrary bytes

### Requirement: Two-tier fuzz schedule

Fuzz targets SHALL run in two tiers: a 30-second smoke tier in `nix flake check` for PR gating, and a 10-minute nightly tier for deeper coverage.

#### Scenario: Smoke tier runs in flake check

- **WHEN** `nix flake check` is executed
- **THEN** each snix fuzz target SHALL run for 30 seconds
- **AND** the check SHALL fail if any target finds a crash

#### Scenario: Nightly tier available as explicit target

- **WHEN** `nix build .#fuzz-nightly-snix-nar` is executed
- **THEN** the NAR fuzz target SHALL run for 10 minutes

### Requirement: Seed corpus for fuzz targets

Each fuzz target SHALL have a seed corpus containing valid inputs to bootstrap coverage.

#### Scenario: NAR fuzz has real NAR seed

- **WHEN** the NAR fuzz target starts
- **THEN** the corpus directory SHALL contain at least one valid NAR archive
