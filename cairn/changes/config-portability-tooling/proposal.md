## Why

Molten's current configuration works on the primary developer machine, but several reviewed config surfaces still encode local absolute paths, duplicate dependency source pins, and a floating Rust nightly. That makes the config harder to reproduce on another OnixResearch checkout and harder to treat as release-review evidence.

The repository should make its development, hook, and Nix configuration relocatable while preserving the existing evidence boundaries: runtime code consumes checked artifacts and receipts, not live host configuration.

## What Changes

- Replace user-specific absolute paths in repo-owned hook/Nix configuration with workspace-relative defaults, explicit environment variables, or reviewed flake inputs.
- Pin the Rust toolchain used by release and CI evidence to a dated toolchain instead of a floating `nightly`.
- Add a deterministic drift check that compares Cargo git dependency revisions with the Nix local-source map used by unit2nix.
- Add a lightweight config lint/readback check for forbidden local paths, floating release toolchains, placeholder refs in release-scoped config, and unexplained repeated config constants.
- Split the largest Nix check constants or helper modules enough that VM addresses, attempt counts, event limits, and timeout values are named and review-visible.

## Impact

- **Files**: `flake.nix`, `flake.lock` inputs, `.pre-commit-config.yaml`, `rust-toolchain.toml`, README/operator guidance, and a focused config lint/check surface.
- **Testing**: positive checks for default local checkout behavior and negative checks for hard-coded user paths, mismatched source pins, and floating release toolchains.
- **Safety**: portability checks are authoring/release evidence only. They do not grant runtime authority, policy, provenance, resource, transport, source-gate, retention, or execution trust.
