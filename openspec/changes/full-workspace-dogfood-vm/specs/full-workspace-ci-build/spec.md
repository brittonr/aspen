## ADDED Requirements

### Requirement: Full workspace source pushed to Forge

The test SHALL push all 80 Aspen workspace crates to a Forge repository via git-remote-aspen. The source tree SHALL be derived from the existing `fullSrc` Nix derivation, which includes vendored openraft, stubbed git dependencies, and patched Cargo.lock. The git repository SHALL contain a valid Cargo workspace with all 80 members resolvable.

#### Scenario: Source tree integrity after push

- **WHEN** the `fullSrc`-derived source tree is committed and pushed to Forge
- **THEN** the Forge repository SHALL contain at least 80 directories under `crates/`, a root `Cargo.toml` with workspace members, a `Cargo.lock`, and the vendored `openraft/` directory

#### Scenario: CI config included in push

- **WHEN** the source tree is pushed
- **THEN** the repository SHALL contain `.aspen/ci.ncl` with a single-stage pipeline containing one `type = 'nix` job targeting the full workspace build

### Requirement: CI pipeline auto-triggers and builds full binary

The CI pipeline SHALL auto-trigger on push and execute a `nix build` job via NixBuildWorker that compiles the full `aspen-node` binary from the 80-crate workspace. The build SHALL use `rustPlatform.buildRustPackage` with features `ci,docs,hooks,shell-worker,automerge,secrets,forge,git-bridge,blob`. The pipeline SHALL complete with status `success`.

#### Scenario: Pipeline triggers within 60 seconds

- **WHEN** the source is pushed to Forge with CI watch enabled
- **THEN** a pipeline run SHALL appear in `ci list` within 60 seconds

#### Scenario: Nix build compiles all 80 crates

- **WHEN** the NixBuildWorker executes the `nix build` job
- **THEN** cargo SHALL compile all 80 workspace crates plus their ~600 external dependencies and produce an `aspen-node` binary

#### Scenario: Pipeline completes successfully

- **WHEN** the build job finishes
- **THEN** the pipeline status SHALL be `success` with zero failed jobs

### Requirement: Build logs stream during compilation

The test SHALL stream build logs via `ci logs --follow` while the build is running. The streamed logs SHALL contain cargo compilation output.

#### Scenario: Logs captured with nonzero size

- **WHEN** the build job completes
- **THEN** the captured log file SHALL be at least 10,000 bytes (full workspace cargo output is substantial)

### Requirement: CI-built binary is executable

The test SHALL extract the output path from the completed job and execute the CI-built `aspen-node` binary. The binary SHALL respond to `--version` with version information.

#### Scenario: Binary runs successfully

- **WHEN** the CI-built `aspen-node --version` is executed
- **THEN** the output SHALL contain `aspen` and a version string, and the exit code SHALL be 0

### Requirement: Build artifacts stored in blob and cache

The CI system SHALL upload the built store path as a NAR to iroh-blobs and register it in the nix binary cache. The cache entry SHALL be queryable and its blob hash and NAR size SHALL match the upload record.

#### Scenario: Blob upload verified

- **WHEN** the build completes
- **THEN** the job result SHALL contain `uploaded_store_paths` with a nonzero `nar_size` and a valid `blob_hash`, and `cache_registered` SHALL be `true`

#### Scenario: Cache query returns matching entry

- **WHEN** the test queries `cache query <output_path>`
- **THEN** the result SHALL have `was_found = true` with `blob_hash` and `nar_size` matching the upload record

### Requirement: VM resource allocation sufficient for full build

The VM SHALL be configured with at least 8192 MB RAM, 40 GB disk, 2+ cores, `writableStoreUseTmpfs = false`, and `nix.settings.sandbox = false`. The pipeline timeout SHALL be at least 1800 seconds.

#### Scenario: Build does not OOM or exhaust disk

- **WHEN** the full workspace build runs inside the VM
- **THEN** the build SHALL complete without out-of-memory kills or "No space left on device" errors
