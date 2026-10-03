# Workspace and toolchain reference

Mode: Reference

Use this page to locate the owner of a development setting and understand what an observed artifact means. It is a source snapshot, not a claim that dependencies are currently fetchable or that a build succeeded. No commands were executed for this page. Return to the [Handbook](../README.md).

## Workspace targets

The [root manifest](../../../Cargo.toml) declares four members and Cargo resolver `3`. Workspace package defaults are edition `2024` and license `AGPL-3.0-or-later`. These are package metadata; license metadata alone does not establish distribution compliance.

| Package | Manifest-owned entry or role | Selection caution |
| --- | --- | --- |
| `molten` | Library `src/lib.rs`; default binary `src/main.rs` named `molten` | Library tests and application subcommands are different surfaces |
| `molten-core` | Library `crates/molten-core/src/lib.rs` | Root-package nextest filters do not include it by name |
| `molten-node-host` | Internal capability-rooted node host package | Has its own manifest and development dependencies |
| `molten-release-policy` | Binary `crates/molten-release-policy/src/main.rs`; `publish = false` | Observes release inputs; not the application binary |

The root also declares `molten-native-extension-fixture`, an additional binary, and a `doltlite_oracle` integration test requiring the `doltlite-oracle` feature. Do not infer that a default run selects the fixture binary or that default tests enable the oracle.

## Toolchain and development environment

| Setting or tool | Actual owner | Review use |
| --- | --- | --- |
| `nightly-2026-05-26` | `rust-toolchain.toml` | Main Rust toolchain identity |
| `rust-src`, `rustfmt`, `clippy` | Toolchain components | Available components, not evidence they ran |
| Nix Rust toolchain | `flake.nix`, using `fromRustupToolchainFile` | Keeps shell/build toolchain tied to the file |
| `git-fetch-with-cli = true` | `.cargo/config.toml` | Cargo Git fetching uses the CLI path |
| Nextest `0.9.136` required/recommended | `.config/nextest.toml` | Runner compatibility requirement |
| Steel, Nickel, Wasmtime | Default Nix development shell | Language/runtime tooling, not an installed application |
| `cargo-nextest`, `cargo-watch`, `rust-analyzer` | Default development shell | Development tools |
| unit2nix, `nix-prefetch-git`, `jq` | Default development shell | Build-plan and source-hash workflow tools |
| `cargo-octet` | Catalogued `octet-toolchain` flake input | An unrelated ambient executable is not equivalent evidence |

The shell also supplies `pkg-config`, Clang, and Mold. Its Flux profiler CLI addition is conditional on `x86_64-linux`. Installing a similarly named system package does not demonstrate identity with the flake's selected tool.

## Root feature switches

| Feature | Manifest effect or boundary |
| --- | --- |
| `doltlite-oracle` | Enables the optional oracle surface and required-feature test selection |
| `executable-extents` | Enables data encoding, executable-extent dependencies, and the core feature |
| `world-snapshot-vm-cohort` | Enables optional `vm-cohort-core` dependency |
| `profiler` | Enables optional Flux dependency |
| `profiler-alloc` | Adds allocation profiling through `profiler` |
| `profiler-perf` | Adds Flux perf support through `profiler` |
| `profiler-disabled` | Enables Flux's disabled-profiling configuration |

These are compile-time selections, not runtime admission grants. Optional adapters still have their own source, platform, and policy prerequisites. A feature name does not establish that its live composition is available in a particular environment.

## Nextest selection and artifacts

All named semantic profiles below restrict selection to `package(molten)`. Inspect their full filter expressions in [the configuration](../../../.config/nextest.toml) when changing names or moving tests.

| Profile | Main selection intent | Important distinction |
| --- | --- | --- |
| `ci` | Root package | Four test threads; 45-minute global timeout |
| `deterministic` | Root package excluding live/VM/dogfood/soak/exploratory names | Four threads; 20-minute global timeout |
| `fast-core` | Hardening/bounded/Preserves/profile/receipt names | Inherits deterministic policy, not all core-crate tests |
| `harness` | Harness/replay/repro/gate/receipt names | Name filter, not every test helper |
| `cli` | CLI/cliharness/command/receipt names | Excludes the non-deterministic name partitions |
| `distributed-simulation` | Distributed/simulation/fault/two-peer/remote names | Does not imply live transport coverage |
| `vm-platform` | VM/NixOS/platform names | Platform-scoped; not itself a VM launcher |
| `dogfood-soak` | Dogfood/soak/release names | Sixty-minute global timeout |
| `exploratory` | Root package | One retry; flaky result passes; diagnostic distinction matters |

Named profiles configure JUnit at `target/nextest/<profile>/junit.xml`. JUnit is a rendered report. The README separately describes the hermetic nextest check preserving `cargo-metadata.json`, `binaries-metadata.json`, `junit.xml`, and `ci-test-run-receipt.preserves`; a local Cargo test run does not promise those outputs.

## Worked ownership lookup

Suppose `cargo nextest run --profile fast-core` succeeds but a change touched `molten-node-host`. The profile's name is not evidence of package coverage: its filter is explicitly `package(molten)`. The correct review question is which selected tests exercise the changed boundary and whether the owning member's tests were separately covered. Record the actual selection rather than relabeling the pass as a workspace-wide result.

Dependency changes have another multi-owner boundary. The Nickel release profile, Cargo manifests and lock, Nix input and lock, metadata-free source hashes, and both generated build plans must agree. The [governing update sequence](../../reproducible-dependencies.md) owns that process; this reference intentionally does not prescribe ad hoc lockfile edits or override-based release evidence.

## Sources

- [Handbook](../README.md)
- [Reproducible dependencies](../../reproducible-dependencies.md)
- [Dependency cohort theory](../../technical/engineering/dependency-cohorts-and-reproducible-builds.md)
- [Root manifest](../../../Cargo.toml)
- [Core manifest](../../../crates/molten-core/Cargo.toml)
- [Node host manifest](../../../crates/molten-node-host/Cargo.toml)
- [Release-policy manifest](../../../crates/molten-release-policy/Cargo.toml)
- [Toolchain](../../../rust-toolchain.toml), [Cargo configuration](../../../.cargo/config.toml), [flake](../../../flake.nix), and [nextest configuration](../../../.config/nextest.toml)
