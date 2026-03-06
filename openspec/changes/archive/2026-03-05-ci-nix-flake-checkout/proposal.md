## Why

CI checkout directories are bare file extractions with no `.git/` directory. Nix flakes require a git repository context to evaluate — `nix build .#pkg` fails immediately because Nix can't find the flake source. This blocks every Nix-based CI build, making the entire forge→CI→cache pipeline non-functional for its primary use case: building Aspen itself.

## What Changes

- After extracting files from Forge git objects, initialize a minimal git repository in the checkout directory so Nix flakes can evaluate
- Wire SNIX services (`snix_blob_service`, `snix_directory_service`, `snix_pathinfo_service`) into `NixBuildWorkerConfig` in the node binary so the SNIX cache upload path is functional
- Expose `publish_to_cache` in the Nickel CI config schema so pipeline authors can control cache publishing per-job

## Capabilities

### New Capabilities

- `ci-flake-checkout`: Git repository initialization in CI checkout directories for Nix flake compatibility

### Modified Capabilities

- `ci`: Wire SNIX services into NixBuildWorkerConfig and expose publish_to_cache in pipeline config schema

## Impact

- `crates/aspen-ci/src/checkout.rs` — add git init after file extraction
- `crates/aspen-ci-executor-nix/src/executor.rs` — plumb publish_to_cache from JobConfig
- `src/bin/aspen_node/setup/client.rs` — wire SNIX services into NixBuildWorkerConfig
- `crates/aspen-ci/src/config/` — add publish_to_cache to Nickel schema
- Requires `git` available in CI execution environment (already present in dogfood-node.nix)
