## Why

The current `ci-dogfood-workspace-test` only builds 18 of 80 crates (9% of the codebase — 32K of 343K lines). The remaining 62 crates include every heavy subsystem: raft consensus (30K lines), CLI (29K), coordination (21K), jobs (20K), cluster (17K), forge (16K). We already compile the full workspace on the host via `nix build .#aspen-node`, but that binary is pre-installed into the VM — it never passes through the Forge → CI → nix build self-hosting loop. Until the full workspace compiles through our own CI pipeline inside a VM, the dogfood claim has an asterisk.

## What Changes

- New NixOS VM integration test (`ci-dogfood-full-workspace-test`) that pushes all 80 crates to Forge, triggers the CI pipeline, and compiles the entire `aspen-node` binary via `NixBuildWorker` inside the VM.
- The test reuses the existing `fullSrc` derivation (already packages all 80 crates + vendored openraft + patched Cargo.lock) instead of manually copying each crate's source tree. This sidesteps the 18-crate wall the current workspace test hit.
- The inner flake uses `craneLib.buildPackage` (or `rustPlatform.buildRustPackage`) with the same feature set as the production `aspen-node` build: `ci,docs,hooks,shell-worker,automerge,secrets,forge,git-bridge,blob`.
- External git dependencies (iroh-proxy-utils, aspen-wasm-plugin) are handled the same way the host Nix build handles them — stubbed or vendored into the source tree before push.
- The CI pipeline config (`.aspen/ci.ncl`) is a single-stage `type = 'nix` job targeting the full binary.
- After the pipeline succeeds, the test runs the CI-built `aspen-node --version` to prove the binary works.
- Verification subtests: blob upload, nix binary cache entry, SNIX decomposition, streamed build logs.

## Capabilities

### New Capabilities

- `full-workspace-ci-build`: NixOS VM test proving the full 80-crate Aspen workspace compiles through the Forge → CI → nix build self-hosting pipeline, including external dep vendoring, feature flag propagation, and artifact storage verification.

### Modified Capabilities

## Impact

- `nix/tests/ci-dogfood-full-workspace.nix` — new test file
- `nix/tests/fixtures/` — new fixture files (flake, CI config, possibly a workspace Cargo.lock)
- `flake.nix` — new check entry `ci-dogfood-full-workspace-test`
- VM resource requirements: 8+ GB RAM, 40+ GB disk (full Rust toolchain + 658 crates + link step)
- Build time: estimated 15-30 minutes inside the VM (vs ~4 min on host with cached deps)
- Depends on `fullSrc` derivation already used by host `aspen-node` build
