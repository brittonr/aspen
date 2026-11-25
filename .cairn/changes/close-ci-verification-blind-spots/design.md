# Design

## Workspace Test Coverage in Nextest and Nix

- Update `.config/nextest.toml` so `profile.ci` and `profile.deterministic` cover all workspace packages (`package(molten) | package(molten-core) | package(molten-node-host) | package(molten-release-policy)`), retaining the deterministic exclusion of `live|vm|dogfood|soak|exploratory` partitions on `profile.deterministic`.
- Add explicit `checks.molten-core` and `checks.molten-release-policy` derivations (or expand `checks.nextest` to compile and execute all workspace member test binaries) in `flake.nix` so `nix flake check` executes all 798 `molten-core` tests and all 7 `molten-release-policy` tests hermetically.
- Update `docs/distributed-testing.md` and `AGENTS.md` to match the exact workspace filter semantics without claiming coverage that does not execute.

## Hermetic Workspace Octet and Pinned Cairn Validation

- Update `[workspace.metadata.octet]` in `Cargo.toml` to include `-p molten-core` and `-p molten-release-policy` alongside `molten` and `molten-node-host`.
- Wire a hermetic `checks.octet-workspace` check in `flake.nix` using a single pinned Octet toolchain over the real workspace crates rather than relying exclusively on 19 partial `checks/*-octet` copies, and either wire or delete the two orphaned directories (`checks/content-replication-octet`, `checks/dag-sync-octet`).
- Wire `checks.cairn-validate-strict` into `flake.nix` using the pinned Cairn flake input and its matching generated policy export rather than an unpinned sibling `../cairn` checkout.

## Optional Feature Matrix, Ignored Tests, and Pure Core Hegel Properties

- Add a hermetic feature-check derivation in `flake.nix` (and CI) compiling and running unit tests for all self-contained optional features (`chaoscontrol-conformance`, `job-dag-construction`, `component-rpc`, `component-rpc-telemetry`, `executable-extents`, `marble-object-store`, `spacewasm-pilot`, `world-snapshot-vm-cohort`, `content-store-object-storage`, `profiler*`).
- Audit all 49 `#[ignore]` tests in `src/` and `tests/`: promote hermetic/same-host tests into explicit CI profiles (such as `outside-in` or feature-scoped checks) and require a structured machine-readable reason tag on every remaining `#[ignore]` test explaining its external prerequisite.
- Add `hegeltest` as a dev-dependency of `crates/molten-core` and author bounded `#[hegel::test]` properties covering canonical identity encoding, world-merge three-way laws, lease/reservation conservation, and Prolly tree balancing invariants.

## Pure Core vs Shell Boundaries and Non-Claims

- Nextest profile contracts, feature-coverage manifests, and ignored-test classification checks remain pure declarative/deterministic validators; Nix derivations and GitHub workflow runners remain imperative execution shells.
- Expanding CI rails closes local/hermetic coverage gaps only and grants no runtime authority, external host qualification, or release admission.
