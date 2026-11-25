## Why

Aspen's CI and hermetic Nix verification rails miss major portions of the actual workspace:

1. Every named profile in `.config/nextest.toml` (including `profile.ci` and `profile.deterministic`) hardcodes `default-filter = 'package(molten)...'`. `flake.nix` exposes test checks only for `molten` and `molten-node-host`. As a result, all 798 unit tests in `crates/molten-core` (23.3% of workspace tests, owning Aspen's pure decision cores) and all 7 tests in `crates/molten-release-policy` never execute in `nix flake check` or GitHub Actions.
2. `Cargo.toml` sets `[workspace.metadata.octet] default_scope = ["-p", "molten", "-p", "molten-node-host"]`, silently excluding `molten-core` and `molten-release-policy`. Meanwhile `nix flake check` runs Octet only on 19 isolated `checks/*-octet` micro-crates (leaving 2 orphaned directories unwired) and never runs Octet on `molten`, `molten-node-host`, or `molten-release-policy`.
3. `cairn validate --strict` is absent from `nix flake check` and GitHub Actions, running only in a non-strict local pre-push hook against an unpinned sibling checkout (`../cairn`).
4. 13 of 14 optional Cargo features (including `chaoscontrol-conformance`, `job-dag-construction`, `component-rpc`, `executable-extents`, `spacewasm-pilot`, `world-snapshot-vm-cohort`, and `doltlite-oracle`) and 49 `#[ignore]` integration/outside-in tests never compile or run in default CI.
5. `crates/molten-core` contains zero `#[hegel::test]` properties despite `docs/proof-workflow.md` requiring Hegel property tests for pure core invariants.

## What Changes

- Expand `profile.ci` and `profile.deterministic` in `.config/nextest.toml` (and `checks.nextest` plus dedicated workspace member test checks in `flake.nix`) so `molten-core` and `molten-release-policy` tests execute unconditionally in `nix flake check` and GitHub Actions.
- Extend workspace Octet scope to cover all workspace crates (`molten`, `molten-core`, `molten-node-host`, `molten-release-policy`), wire a hermetic workspace Octet check and pinned strict `cairn validate` check into `flake.nix`, and retire or reconcile orphaned `checks/*-octet` micro-workspaces.
- Add explicit CI compile/test rails for optional Cargo features and categorize the 49 `#[ignore]` tests into runnable hermetic/same-host CI rails versus explicitly environment-blocked external cohorts.
- Add bounded `#[hegel::test]` property suites for `crates/molten-core` pure state-machine, merge, lease, and identity invariants.

## Impact

- **Files**: `.config/nextest.toml`, `Cargo.toml`, `flake.nix`, `.github/workflows/reproducible-dependencies.yml`, `checks/*-octet/`, `crates/molten-core/`, `docs/distributed-testing.md`, and `AGENTS.md`.
- **Verification**: `nix flake check` and GitHub Actions will fail closed on regressions in `molten-core`, `molten-release-policy`, workspace Octet policy, strict Cairn validation, and optional feature compilation.
- **Non-claims**: Passing expanded CI rails establishes hermetic build, test, lint, and structural validation for the exact checked candidate; it does not establish cross-host, physical hardware, or production release qualification.

## Out of Scope

- Replacing synthetic VM or placeholder evidence receipts (tracked separately in `eliminate-self-certifying-evidence-gates`).
- Rewriting `tools/tracey/inherited_debt_guard.rs` or proof-composition verifiers (tracked separately in `harden-traceability-proof-composition-and-static-audits`).
- Executing external cloud S3, physical TPM2, or multi-host hardware qualification campaigns that require unavailable external infrastructure.
