## 1. Dependency and Feature Flag Setup

- [x] 1.1 Add `snix-build`, `snix-eval`, `snix-glue`, `snix-serde`, `nix-daemon`, `nar-bridge`, `snix-castore-http`, `snix-tracing` to workspace `Cargo.toml` dependencies (same git rev as existing snix deps)
- [x] 1.2 Define new feature flags in workspace `Cargo.toml`: `snix-http`, `snix-daemon`, `snix-eval`, `snix-build`
- [x] 1.3 Add `nix-cli-fallback` feature flag to `aspen-ci-executor-nix` for subprocess retention
- [x] 1.4 Update `full` feature to include all new snix features
- [x] 1.5 Verify `cargo check --features full` compiles with all new dependencies

## 2. Tier 1: nar-bridge Adoption (Replace Cache Gateway)

- [x] 2.1 Add `nar-bridge` dependency to `aspen-nix-cache-gateway` behind `snix-http` feature
- [x] 2.2 Create adapter to construct `nar_bridge::AppState` from Aspen's `BlobService`/`DirectoryService`/`PathInfoService`
- [x] 2.3 Replace custom hyper HTTP server in `aspen-nix-cache-gateway/src/server.rs` with nar-bridge axum router
- [x] 2.4 Integrate cluster signing key into nar-bridge's narinfo response pipeline
- [x] 2.5 Add PUT endpoint support (narinfo + NAR upload) via nar-bridge
- [x] 2.6 Remove custom hyper/http-body-util dependencies from `aspen-nix-cache-gateway`
- [x] 2.7 Update `aspen-nix-cache-gateway/src/main.rs` to use nar-bridge initialization
- [x] 2.8 Port existing `gateway_test.rs` to validate nar-bridge-backed gateway serves same protocol
- [x] 2.9 Test range request support on NAR downloads

## 3. Tier 1: FlakeRef Parsing

- [x] 3.1 Add `nix-compat` `flakeref` feature/module usage to `aspen-ci-executor-nix`
- [x] 3.2 Replace `NixBuildPayload::flake_ref()` string concatenation with `FlakeRef` parsing and formatting
- [x] 3.3 Add flake URL validation in `NixBuildPayload::validate()` using `FlakeRef` parser
- [x] 3.4 Extract `rev`/`ref` from parsed `FlakeRef` for cache key computation
- [x] 3.5 Write tests for GitHub, GitLab, Path, Git, and Indirect flake ref round-trips

## 4. Tier 1: snix-tracing Adoption

- [x] 4.1 Add `snix-tracing` dependency to `aspen-snix-bridge`
- [x] 4.2 Replace manual `tracing_subscriber` setup in `aspen-snix-bridge/src/main.rs` with `snix-tracing::TracingBuilder`
- [x] 4.3 Integrate snix-tracing clap args into bridge CLI parser
- [x] 4.4 Verify OTLP export works when `OTEL_EXPORTER_OTLP_ENDPOINT` is set

## 5. Tier 1: snix-castore-http Browse

- [x] 5.1 Add `snix-castore-http` dependency to `aspen-snix-bridge` behind `snix-http` feature
- [x] 5.2 Add `--browse` and `--browse-port` CLI flags to `aspen-snix-bridge`
- [x] 5.3 Wire `snix-castore-http` router to serve store path contents when `--browse` is passed
- [x] 5.4 Test directory listing, file download, and symlink redirect via HTTP browser

## 6. Tier 2: nix-daemon Protocol

- [x] 6.1 Add `nix-daemon` crate dependency to `aspen-snix-bridge` behind `snix-daemon` feature
- [x] 6.2 Implement `NixDaemonIO` trait backed by Aspen's `BlobService`/`DirectoryService`/`PathInfoService`
- [x] 6.3 Wire `query_path_info` to look up store paths via PathInfoService
- [x] 6.4 Wire `is_valid_path` to check existence in PathInfoService
- [x] 6.5 Wire `add_to_store_nar` to ingest NARs via `ingest_nar_and_hash` into Aspen's store
- [x] 6.6 Wire `query_valid_paths` for batch validation
- [x] 6.7 Add `--daemon-socket` CLI flag to `aspen-snix-bridge`
- [x] 6.8 Start nix-daemon listener on the Unix socket alongside the gRPC listener
- [x] 6.9 Ensure daemon and gRPC share the same service instances
- [x] 6.10 Test: `nix path-info --store unix:///tmp/aspen-daemon.sock` against Aspen store
- [x] 6.11 Test: `nix copy --to unix:///tmp/aspen-daemon.sock` uploads a store path

## 7. Tier 2: Derivation Analysis

- [x] 7.1 Add `nix-compat` derivation parsing to `aspen-ci-executor-nix`
- [x] 7.2 Create `derivation.rs` module with functions to parse `.drv` files from store paths
- [x] 7.3 Implement build graph construction from `input_derivations` fields
- [x] 7.4 Implement content-hash-based cache key from derivation inputs + environment
- [x] 7.5 Implement closure computation by traversing `references` in PathInfo entries
- [x] 7.6 Integrate cache key check into `NixBuildWorker::execute_build` to skip cached builds
- [x] 7.7 Write tests for linear, diamond, and cyclic dependency graph scenarios

## 8. Tier 3: snix-eval Integration

- [x] 8.1 Add `snix-eval` and `snix-glue` dependencies behind `snix-eval` feature
- [x] 8.2 Create `aspen-ci-executor-nix/src/eval.rs` module for in-process evaluation
- [x] 8.3 Wire `SnixStoreIO` from `snix-glue` to Aspen's `BlobService`/`DirectoryService`/`PathInfoService`
- [x] 8.4 Implement flake evaluation: parse `flake.nix`, resolve inputs from `flake.lock`, evaluate attribute
- [x] 8.5 Implement pre-flight validation: evaluate flake and report errors before queuing builds
- [x] 8.6 Implement derivation-to-`BuildRequest` conversion using snix-glue utilities
- [x] 8.7 Add subprocess fallback: detect IFD and fall back to `nix eval` when `nix-cli-fallback` enabled
- [x] 8.8 Test evaluation of Aspen's own `flake.nix` to verify coverage for dogfood pipeline
- [x] 8.9 Test evaluation error reporting (syntax errors, undefined variables, type mismatches)

## 9. Tier 3: snix-serde Config

- [x] 9.1 Add `snix-serde` dependency behind `snix-eval` feature (transitive dep on snix-eval)
- [x] 9.2 Create `aspen-ci/src/nix_config.rs` for Nix-based pipeline config deserialization
- [x] 9.3 Define `serde::Deserialize` structs for CI pipeline definitions (stages, jobs, attributes)
- [x] 9.4 Implement `.aspen/ci.nix` loading: read file, evaluate with `snix_serde::from_str`, validate
- [x] 9.5 Enforce pure evaluation (no I/O builtins) for config parsing
- [x] 9.6 Write tests: valid config, nested config, type mismatch errors, impure rejection

## 10. Tier 4: snix-build Native Execution

- [ ] 10.1 Add `snix-build` dependency to `aspen-ci-executor-nix` behind `snix-build` feature
- [ ] 10.2 Implement `BuildService` initialization with Aspen-backed castore services
- [ ] 10.3 Wire bubblewrap sandbox: configure `SandboxSpec` from `BuildRequest` fields
- [ ] 10.4 Wire OCI sandbox as fallback when bubblewrap is unavailable
- [ ] 10.5 Integrate build execution: eval → derivation → BuildRequest → do_build → upload outputs
- [ ] 10.6 Connect build output paths to existing SNIX upload pipeline (`upload_store_paths_snix`)
- [ ] 10.7 Add `nix-cli-fallback` gate: when `snix-build` feature disabled, use subprocess path
- [ ] 10.8 Implement build log capture from sandbox stdout/stderr and stream to CI log infrastructure
- [ ] 10.9 Test: build a simple flake (hello world) end-to-end via snix-build
- [ ] 10.10 Test: build Aspen's own flake via snix-build (dogfood validation)
- [ ] 10.11 Test: verify parity between snix-build and `nix build` subprocess for same flake
- [ ] 10.12 Run both execution paths in CI for one release cycle to validate parity

## 11. Cleanup and Documentation

- [ ] 11.1 Remove `aspen-nix-cache-gateway` custom HTTP server code after nar-bridge adoption is validated
- [ ] 11.2 Update `AGENTS.md` with new snix feature flags and their purpose
- [ ] 11.3 Update `docs/` design documents for nix integration architecture
- [ ] 11.4 Add integration test: push flake to Forge → CI builds via snix-build → result served via nar-bridge
- [ ] 11.5 Add NixOS VM test for nix-daemon protocol access to Aspen cluster
