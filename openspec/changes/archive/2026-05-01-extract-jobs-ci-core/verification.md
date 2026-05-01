## Verification

This change is complete pending archive. Inventory, core-crate, downstream fixture, negative boundary, policy/readiness, compatibility, and final OpenSpec preflight tasks are complete.

## Commands

- `cargo tree -p aspen-jobs -e normal --depth 2` — saved in `evidence/i2-baseline-dependencies.md`.
- `cargo tree -p aspen-ci-core -e normal --depth 2` — saved in `evidence/i2-baseline-dependencies.md`.
- `cargo tree -p aspen-jobs-protocol -e normal --depth 2` — saved in `evidence/i2-baseline-dependencies.md`.
- `cargo tree -p aspen-jobs-core -e normal --depth 2` — saved in `evidence/i3-i5-jobs-core-checks.md`.
- `cargo check -p aspen-jobs-core` — passed; saved in `evidence/i3-i5-jobs-core-checks.md` and `evidence/jobs-ci-core-compatibility.txt`.
- `cargo test -p aspen-jobs-core` — passed; saved in `evidence/i3-i5-jobs-core-checks.md` and `evidence/jobs-ci-core-compatibility.txt`.
- `cargo check -p aspen-jobs` — passed; saved in `evidence/i4-aspen-jobs-core-reexport.md` and `evidence/jobs-ci-core-compatibility.txt`.
- `openspec/changes/archive/2026-05-01-extract-jobs-ci-core/fixtures/run-jobs-ci-core-boundary-fixtures.sh` — passed; saved in `evidence/v1-v2-jobs-ci-core-fixtures.txt`.
- `cargo metadata --manifest-path openspec/changes/archive/2026-05-01-extract-jobs-ci-core/fixtures/jobs-ci-core-portable-smoke/Cargo.toml --format-version 1 --no-deps` — passed; saved in `evidence/jobs-ci-core-downstream-metadata.json`.
- Focused compatibility checks for `aspen-jobs-core`, `aspen-jobs-protocol`, `aspen-ci-core`, `aspen-jobs`, `aspen-ci`, `aspen-job-handler`, `aspen-ci-handler`, `aspen-ci-executor-shell`, `aspen-ci-executor-vm`, and `aspen-ci-executor-nix` — passed; saved in `evidence/jobs-ci-core-compatibility.txt`.
- `scripts/check-crate-extraction-readiness.rs --policy docs/crate-extraction/policy.ncl --inventory docs/crate-extraction.md --manifest-dir docs/crate-extraction --candidate-family jobs-ci-core --output-json openspec/changes/archive/2026-05-01-extract-jobs-ci-core/evidence/jobs-ci-core-readiness.json --output-markdown openspec/changes/archive/2026-05-01-extract-jobs-ci-core/evidence/jobs-ci-core-readiness.md` — saved under `evidence/`.
- Final `openspec validate extract-jobs-ci-core --json` and `python ~/.hermes/skills/agentkit-port/openspec/scripts/openspec_helper.py verify extract-jobs-ci-core --json` — saved in `evidence/v5-final-preflight.txt`.

## Task Coverage

- [x] I1 Inventory `aspen-jobs` public exports and classify each as reusable core, compatibility re-export, runtime shell, worker/executor adapter, or deferred orchestration.  
  - Evidence: `openspec/changes/archive/2026-05-01-extract-jobs-ci-core/evidence/i1-jobs-public-surface.md`
- [x] I2 Capture current dependency graphs for `aspen-jobs`, `aspen-ci-core`, and `aspen-jobs-protocol`; identify forbidden dependencies for reusable defaults.  
  - Evidence: `openspec/changes/archive/2026-05-01-extract-jobs-ci-core/evidence/i2-baseline-dependencies.md`
- [x] I3 Add `crates/aspen-jobs-core` and workspace membership with a minimal reusable dependency set for job IDs, priority, retry policy, schedule descriptors, job config/spec/result/status helpers, dependency state, and deterministic run-state helpers.  
  - Evidence: `Cargo.toml`, `Cargo.lock`, `crates/aspen-jobs-core/Cargo.toml`, `crates/aspen-jobs-core/src/lib.rs`, `openspec/changes/archive/2026-05-01-extract-jobs-ci-core/evidence/i3-i5-jobs-core-checks.md`
- [x] I4 Migrate `aspen-jobs` to depend on and re-export the moved core surface without breaking existing `aspen_jobs::*` imports.  
  - Evidence: `crates/aspen-jobs/Cargo.toml`, `crates/aspen-jobs/src/lib.rs`, `openspec/changes/archive/2026-05-01-extract-jobs-ci-core/evidence/i4-aspen-jobs-core-reexport.md`
- [x] I5 Add serialization/roundtrip and deterministic transition tests for the moved core surface.  
  - Evidence: `crates/aspen-jobs-core/src/lib.rs`, `openspec/changes/archive/2026-05-01-extract-jobs-ci-core/evidence/i3-i5-jobs-core-checks.md`
- [x] V1 Add a downstream fixture that depends on `aspen-jobs-core`, `aspen-ci-core`, and `aspen-jobs-protocol` only as reusable defaults; compile it and save metadata/check output.  
  - Evidence: `openspec/changes/archive/2026-05-01-extract-jobs-ci-core/fixtures/jobs-ci-core-portable-smoke/`, `openspec/changes/archive/2026-05-01-extract-jobs-ci-core/evidence/v1-v2-jobs-ci-core-fixtures.txt`, `openspec/changes/archive/2026-05-01-extract-jobs-ci-core/evidence/jobs-ci-core-downstream-metadata.json`
- [x] V2 Add a negative fixture or checker mutation proving reusable defaults cannot import `aspen-jobs`, workers, handlers, root `aspen`, shell/VM/Nix executors, concrete transport, Redb storage, or process runtime APIs.  
  - Evidence: `openspec/changes/archive/2026-05-01-extract-jobs-ci-core/fixtures/jobs-ci-runtime-negative/`, `openspec/changes/archive/2026-05-01-extract-jobs-ci-core/evidence/v1-v2-jobs-ci-core-fixtures.txt`, `openspec/changes/archive/2026-05-01-extract-jobs-ci-core/evidence/jobs-ci-core-forbidden-boundary.txt`
- [x] V3 Update `docs/crate-extraction/jobs-ci-core.md`, `docs/crate-extraction.md`, and `docs/crate-extraction/policy.ncl`; run the extraction-readiness checker for the jobs/CI family and save JSON/Markdown evidence.  
  - Evidence: `docs/crate-extraction/jobs-ci-core.md`, `docs/crate-extraction.md`, `docs/crate-extraction/policy.ncl`, `openspec/changes/archive/2026-05-01-extract-jobs-ci-core/evidence/jobs-ci-core-readiness.json`, `openspec/changes/archive/2026-05-01-extract-jobs-ci-core/evidence/jobs-ci-core-readiness.md`
- [x] V4 Run focused compatibility checks for `aspen-jobs`, `aspen-ci`, job/CI handlers, executor crates, CLI/dogfood consumers as applicable, plus `cargo check -p aspen-jobs-core`; save results.  
  - Evidence: `openspec/changes/archive/2026-05-01-extract-jobs-ci-core/evidence/jobs-ci-core-compatibility.txt`
- [x] V5 Run OpenSpec helper/validate/preflight and save final verification evidence before archiving.  
  - Evidence: `openspec/changes/archive/2026-05-01-extract-jobs-ci-core/evidence/v5-final-preflight.txt`
