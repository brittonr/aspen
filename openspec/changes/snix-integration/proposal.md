## Why

Aspen uses three snix crates (`snix-castore`, `snix-store`, `nix-compat`) but maintains ~1,400 lines of custom code that duplicates functionality already available in the other snix crates. The CI executor shells out to the `nix` binary for builds and evaluation. The nix cache gateway is a hand-rolled HTTP server. These are maintenance liabilities and prevent the self-hosted goal — an Aspen cluster should be a complete Nix build+store infrastructure with zero external dependencies.

The remaining snix crates (`snix-build`, `snix-eval`, `snix-glue`, `snix-serde`, `nix-daemon`, `nar-bridge`, `snix-castore-http`, `snix-tracing`) provide sandboxed build execution, in-process Nix evaluation, the nix-daemon protocol, HTTP binary cache serving, and castore browsing — all backed by the same trait interfaces Aspen already implements.

## What Changes

- Replace `aspen-nix-cache-gateway`'s hand-rolled HTTP server (523 lines) with `nar-bridge` backed by Aspen's existing `BlobService`/`DirectoryService`/`PathInfoService` implementations
- Implement `NixDaemonIO` trait so any Nix client can use Aspen as its store via the daemon socket protocol, eliminating the need for the HTTP cache gateway in local-access scenarios
- Replace `nix build` subprocess invocation in `aspen-ci-executor-nix` with `snix-build`'s `BuildService` trait and bubblewrap/OCI sandbox — builds run without the Nix CLI binary
- Add in-process Nix evaluation via `snix-eval`/`snix-glue` for flake introspection, pre-flight validation, and derivation-to-BuildRequest conversion (required by snix-build)
- Use `nix-compat::flakeref::FlakeRef` for structured flake reference parsing instead of raw string concatenation
- Use `nix-compat::derivation` for build graph analysis and smarter CI cache invalidation
- Adopt `snix-serde` for deserializing Nix expressions into Rust config structs (cluster config, CI pipeline definitions)
- Adopt `snix-tracing` for consistent tracing configuration across snix-dependent crates, replacing per-crate tracing setup
- Use `snix-castore-http` to expose castore contents over HTTP for debugging and browsing store paths

## Capabilities

### New Capabilities

- `snix-daemon-protocol`: Implement nix-daemon protocol so Nix clients can use Aspen as a native store
- `snix-native-builds`: Replace `nix build` subprocess with snix-build's BuildService for CLI-free sandboxed builds
- `snix-eval-integration`: In-process Nix evaluation for flake introspection and derivation resolution
- `snix-flakeref-parsing`: Structured flake reference parsing and validation
- `snix-derivation-analysis`: Parse .drv files for build graph analysis and cache invalidation
- `snix-serde-config`: Deserialize Nix expressions into Rust types for configuration
- `snix-nar-bridge-adoption`: Replace hand-rolled cache gateway HTTP with nar-bridge
- `snix-castore-http-browse`: HTTP browsing of castore contents for debugging
- `snix-tracing-adoption`: Unified tracing configuration for snix-dependent crates

### Modified Capabilities

- `nix-cache-gateway`: HTTP serving moves from custom implementation to nar-bridge; same protocol, different backend
- `ci`: Nix build jobs switch from subprocess to in-process BuildService

## Impact

- **Crates modified**: `aspen-ci-executor-nix`, `aspen-nix-cache-gateway`, `aspen-snix-bridge`, `aspen-snix`, `aspen-castore`, `aspen-nix-handler`, `aspen-cluster`
- **Crates potentially removed**: `aspen-nix-cache-gateway` (replaced by nar-bridge wiring)
- **Dependencies added**: `snix-build`, `snix-eval`, `snix-glue`, `snix-serde`, `nix-daemon`, `nar-bridge`, `snix-castore-http`, `snix-tracing`
- **Dependencies removed**: Manual HTTP server code (hyper/http-body-util in cache gateway)
- **Binary impact**: `aspen-node` gains native nix-daemon socket support; `aspen-ci-agent` no longer requires `nix` CLI on PATH
- **Feature flags**: New `snix-build`, `snix-eval`, `snix-daemon` features; existing `snix` feature expanded
- **Breaking**: CI workers that rely on `nix` CLI in PATH will need migration path (keep subprocess fallback behind feature flag during transition)
