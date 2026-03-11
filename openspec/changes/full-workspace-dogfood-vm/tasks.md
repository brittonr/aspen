## 1. Inner Flake and CI Config Fixtures

- [x] 1.1 Create `nix/tests/fixtures/full-workspace-flake.nix` — the flake.nix that will be injected into the Forge repo. Uses `rustPlatform.buildRustPackage` with `cargoLock.lockFile`, features `ci,docs,hooks,shell-worker,automerge,secrets,forge,git-bridge,blob`, `doCheck = false`, `nativeBuildInputs = [perl pkg-config cmake]` for native deps (ring, aws-lc-rs, redb). Single output: `packages.x86_64-linux.default` producing `aspen-node`.
- [x] 1.2 Create `nix/tests/fixtures/full-workspace-ci.ncl` — single-stage CI config with one `type = 'nix` job, `flake_attr = "packages.x86_64-linux.default"`, `timeout_secs = 1800`.

## 2. Source Tree Derivation

- [x] 2.1 Create a `fullWorkspaceRepo` derivation in the test file that takes `fullSrc` (already has all 80 crates + stubs + patched lockfile), flattens it from `$fullSrc/aspen/` to the repo root, injects the inner flake.nix and `.aspen/ci.ncl` fixtures, and strips `.cargo/config.toml` vendor overrides (the inner nix build handles vendoring via `cargoLock.lockFile`). Verify the derivation contains `crates/`, `openraft/`, `Cargo.toml`, `Cargo.lock`, and at least 80 subdirectories under `crates/`.

## 3. NixOS VM Test File

- [x] 3.1 Create `nix/tests/ci-dogfood-full-workspace.nix` with the VM node configuration: `memorySize = 8192`, `cores = 4`, `diskSize = 40960`, `writableStoreUseTmpfs = false`, `nix.settings.sandbox = false`, `nix.settings.experimental-features = ["nix-command" "flakes"]`, `nix.registry.nixpkgs.flake = nixpkgsFlake`. Uses `full-aspen-node-plugins` for the node package (same as other dogfood tests).
- [x] 3.2 Write the test script with helper functions (`get_ticket`, `cli`, `cli_text`, `plugin_cli`, `stream_job_logs`, `wait_for_pipeline`) matching the pattern from existing dogfood tests.
- [x] 3.3 Implement boot sequence: `start_all`, `wait_for_unit`, `wait_for_file` (cluster ticket), `cluster health`, `cluster init`, WASM plugin install (kv + forge).
- [x] 3.4 Implement Forge push: create repo, `ci watch`, copy `fullWorkspaceRepo` to `/tmp`, `git init`, `git add -A`, `git commit`, `git push aspen main`. Assert file count (>=80 crate dirs, >=200 .rs files).
- [x] 3.5 Implement pipeline wait: poll `ci list` for run_id, then `wait_for_pipeline` with 1800s timeout. Stream logs via `ci logs --follow` as jobs are assigned. Assert pipeline status = `success`.
- [x] 3.6 Implement verification subtests: (a) logs captured with >=10KB, (b) extract output_path from job KV data, (c) blob upload verified (nar_size > 0, blob_hash present, cache_registered), (d) `cache query` returns matching entry, (e) run `aspen-node --version` from the output path and assert it contains version info.

## 4. Flake Integration

- [x] 4.1 Add `ci-dogfood-full-workspace-test` to `flake.nix` checks, passing `fullSrc` (or `fullRawSrc`), `bins.full-aspen-node-plugins`, `bins.full-aspen-cli-e2e`, `bins.full-aspen-cli-plugins`, `bins.full-git-remote-aspen`, `nixpkgsFlake`, and the WASM plugin packages.

## 5. Smoke Test

- [x] 5.1 Run `nix build .#checks.x86_64-linux.ci-dogfood-full-workspace-test --impure --option sandbox false -L` and verify all subtests pass. Capture full build log including cargo compilation output showing all 80 crates compiled.
