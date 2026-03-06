## 1. Git Repository Initialization in Checkout

- [x] 1.1 Add `init_git_for_flake(checkout_dir, commit_hash)` function to `crates/aspen-ci/src/checkout.rs` — runs `git init`, `git add -A`, `git commit` with Forge commit hash in message
- [x] 1.2 Add git binary pre-flight check — verify `git --version` succeeds, return actionable error if missing (included in init_git_for_flake)
- [x] 1.3 Call `init_git_for_flake()` from `prepare_for_ci_build()` as final step (after cargo config patching)
- [x] 1.4 Unit tests: git init creates `.git/`, commit exists, commit message contains hash, files tracked, prepare_for_ci_build integration

## 2. Wire SNIX Services into NixBuildWorkerConfig

- [x] 2.1 In `src/bin/aspen_node/setup/client.rs`, construct `IrohBlobService`, `RaftDirectoryService`, `RaftPathInfoService` when snix feature is enabled and `config.snix.is_enabled`
- [x] 2.2 Pass SNIX service Arcs to `NixBuildWorkerConfig` fields (`snix_blob_service`, `snix_directory_service`, `snix_pathinfo_service`)
- [x] 2.3 Verify `NixBuildWorkerConfig::validate()` returns true with SNIX services wired (all-or-none construction matches validate's expectation)
- [x] 2.4 Compile-test all feature combos: snix on, snix off, ci without snix (all clean)

## 3. Expose publish_to_cache in Pipeline Config

- [x] 3.1 Add `publish_to_cache: bool` field to `JobConfig` in `crates/aspen-ci-core/src/config/types.rs` with serde default `true`
- [x] 3.2 Plumb `job.publish_to_cache` into `NixBuildPayload::publish_to_cache` in `build_nix_payload()` in `crates/aspen-ci/src/orchestrator/pipeline/executor.rs`
- [x] 3.3 Add `publish_to_cache` to the Nickel CI schema (`ci_schema.ncl`) as optional boolean defaulting to true
- [x] 3.4 Unit test: JobConfig with publish_to_cache false propagates to NixBuildPayload
