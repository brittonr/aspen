## ADDED Requirements

### Requirement: Non-Rust include_str files trigger Nix rebuilds

When a file referenced by `include_str!` or `include_bytes!` changes, the Nix derivation for the containing crate SHALL produce a new output hash, causing downstream derivations to rebuild.

#### Scenario: flake_compat_bundled.nix change triggers rebuild

WHEN `flake_compat_bundled.nix` is modified (content changes)
THEN `nix build .#checks.x86_64-linux.snix-flake-native-build-test` SHALL build a new `aspen-node` binary
AND the new binary SHALL contain the updated flake-compat content (verified via `strings`)

#### Scenario: No-op change does not trigger rebuild

WHEN `flake_compat_bundled.nix` is unchanged between two builds
THEN the Nix derivation SHALL be served from cache
AND no recompilation SHALL occur

### Requirement: build.rs tracks include_str dependencies

Crates using `include_str!` on non-Rust files SHALL have a `build.rs` that emits `cargo:rerun-if-changed=<path>` for each included file.

#### Scenario: build.rs emits rerun-if-changed

WHEN `aspen-ci-executor-nix` is compiled
THEN the build script SHALL emit `cargo:rerun-if-changed=src/flake_compat_bundled.nix`
AND cargo SHALL recompile the crate when the `.nix` file changes
