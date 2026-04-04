## Why

The dogfood pipeline (`nix run .#dogfood-local -- full`) fails at the CI clippy stage. The pipeline starts a cluster, pushes source to Forge, triggers CI, and the CI executor's clippy job fails — but the same `nix build .#checks.x86_64-linux.clippy` succeeds when run directly on the repo. Two problems compound here: (1) the CI executor's nix build environment for Forge-checked-out source doesn't match the real repo's flake evaluation context, and (2) the failure log is empty — `fetch_failure_logs` returns `job 'clippy' failed:` with no actual clippy output, making diagnosis impossible.

## What Changes

- Fix the CI nix build environment so clippy (and other flake checks) succeed when run on source pushed to Forge
- Improve CI job failure log capture so actual build errors are surfaced in dogfood output
- Add diagnostic context to dogfood pipeline failures (working directory, flake ref, exit code)

## Capabilities

### New Capabilities

- `ci-forge-nix-compat`: Ensure nix flake checks execute correctly on Forge-checked-out source (flake.lock resolution, input substitution, ciSrc filtering)
- `ci-failure-diagnostics`: Surface actionable build logs from failed CI jobs through the dogfood pipeline and CiGetJobLogs RPC

### Modified Capabilities

## Impact

- `crates/aspen-ci-executor-nix/` — build execution, working directory setup, flake evaluation for Forge checkouts
- `crates/aspen-ci/src/orchestrator/` — job log capture and storage on failure
- `crates/aspen-dogfood/src/ci.rs` — failure log retrieval and display
- `crates/aspen-forge/` — checkout structure compatibility with nix flake evaluation
- `.aspen/ci.ncl` — may need adjustments to clippy job configuration
