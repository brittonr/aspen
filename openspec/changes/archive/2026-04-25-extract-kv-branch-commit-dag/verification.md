# Verification Evidence

## Implementation Evidence

- Changed file: `Cargo.lock`
- Changed file: `crates/aspen-commit-dag/Cargo.toml`
- Changed file: `crates/aspen-commit-dag/src/verified/hash.rs`
- Changed file: `crates/aspen-commit-dag/src/verified/commit_hash.rs`
- Changed file: `crates/aspen-commit-dag/src/verified/mod.rs`
- Changed file: `crates/aspen-commit-dag/src/types.rs`
- Changed file: `crates/aspen-commit-dag/src/store.rs`
- Changed file: `crates/aspen-commit-dag/src/gc.rs`
- Changed file: `docs/crate-extraction.md`
- Changed file: `docs/crate-extraction/kv-branch-commit-dag.md`
- Changed file: `docs/crate-extraction/policy.ncl`
- Changed file: `scripts/check-crate-extraction-readiness.rs`
- Changed file: `openspec/changes/extract-kv-branch-commit-dag/tasks.md`
- Changed file: `openspec/changes/extract-kv-branch-commit-dag/verification.md`
- Changed file: `openspec/changes/extract-kv-branch-commit-dag/fixtures/downstream-branch-dag/Cargo.toml`
- Changed file: `openspec/changes/extract-kv-branch-commit-dag/fixtures/downstream-branch-dag/Cargo.lock`
- Changed file: `openspec/changes/extract-kv-branch-commit-dag/fixtures/downstream-branch-dag/src/lib.rs`
- Changed file: `openspec/changes/extract-kv-branch-commit-dag/evidence/implementation-diff.txt`
- Changed file: `openspec/changes/extract-kv-branch-commit-dag/evidence/summary.txt`
- Changed file: `openspec/changes/extract-kv-branch-commit-dag/evidence/v4-readiness.md`
- Changed file: `openspec/changes/extract-kv-branch-commit-dag/evidence/v4-readiness.json`
- Changed file: `openspec/changes/extract-kv-branch-commit-dag/evidence/v4-negative-mutations-summary.txt`
- Changed file: `openspec/changes/extract-kv-branch-commit-dag/evidence/run-verification.sh`
- Changed file: `openspec/changes/extract-kv-branch-commit-dag/evidence/v4-run-negative-mutations.sh`
- Changed file: `openspec/changes/extract-kv-branch-commit-dag/evidence/openspec-preflight.txt`

## Task Coverage

- [x] R1 Capture baseline compile, dependency, and source-import evidence for `aspen-commit-dag`, `aspen-kv-branch`, and `aspen-kv-branch --features commit-dag`, including current `aspen_raft::verified` import locations and representative consumer feature paths. [covers=kv-branch-commit-dag-extraction.kv-branch-feature-topology,kv-branch-commit-dag-extraction.commit-dag-avoids-raft-compat.commit-dag-graph-excludes-app-shells,kv-branch-commit-dag-extraction.kv-branch-feature-topology.default-avoids-commit-dag,kv-branch-commit-dag-extraction.kv-branch-feature-topology.commit-dag-feature-is-reusable] ✅ completed: 2026-04-25T19:37:10Z
  - Evidence: `openspec/changes/extract-kv-branch-commit-dag/evidence/r1-baseline.md`, `openspec/changes/extract-kv-branch-commit-dag/evidence/r1-baseline-logs/`, `openspec/changes/extract-kv-branch-commit-dag/evidence/r1-prechange-nextest-aspen-commit-dag.txt`

- [x] I1 Move the minimal chain hash helper surface used by `aspen-commit-dag` into the branch/DAG family, update imports away from `aspen_raft::verified`, preserve BLAKE3 commit ID behavior, and remove the normal `aspen-raft` dependency from `crates/aspen-commit-dag/Cargo.toml`. [covers=kv-branch-commit-dag-extraction.commit-dag-avoids-raft-compat,kv-branch-commit-dag-extraction.commit-dag-avoids-raft-compat.hash-helpers-owned-by-family] ✅ completed: 2026-04-25T19:37:10Z
  - Evidence: `crates/aspen-commit-dag/Cargo.toml`, `crates/aspen-commit-dag/src/verified/hash.rs`, `crates/aspen-commit-dag/src/verified/commit_hash.rs`, `crates/aspen-commit-dag/src/verified/mod.rs`, `crates/aspen-commit-dag/src/types.rs`, `crates/aspen-commit-dag/src/store.rs`, `crates/aspen-commit-dag/src/gc.rs`, `openspec/changes/extract-kv-branch-commit-dag/evidence/i1-i2-golden-nextest-aspen-commit-dag.txt`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v1-source-raft-import-grep.txt`

- [x] I2 Add or update tests that compare pre-move and post-move hash behavior for deterministic commit ID inputs, malformed hex input, genesis-parent commits, non-genesis-parent commits, and corrupted mutation hashes. [covers=kv-branch-commit-dag-extraction.commit-dag-avoids-raft-compat.hash-helpers-owned-by-family] ✅ completed: 2026-04-25T19:37:10Z
  - Evidence: `crates/aspen-commit-dag/src/verified/hash.rs`, `crates/aspen-commit-dag/src/verified/commit_hash.rs`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v2-nextest-aspen-commit-dag.txt`

- [x] I3 Add `docs/crate-extraction/kv-branch-commit-dag.md` with candidate metadata, feature contracts, dependency decisions, compatibility plan, representative consumers, and verification rails for both crates. [covers=kv-branch-commit-dag-extraction.inventory-and-policy,kv-branch-commit-dag-extraction.inventory-and-policy.manifest-and-policy-cover-family] ✅ completed: 2026-04-25T19:37:10Z
  - Evidence: `docs/crate-extraction/kv-branch-commit-dag.md`, `openspec/changes/extract-kv-branch-commit-dag/evidence/implementation-diff.txt`

- [x] I4 Add `kv-branch-commit-dag` entries to `docs/crate-extraction/policy.ncl` and update `docs/crate-extraction.md` with the family readiness row and next action. [covers=kv-branch-commit-dag-extraction.inventory-and-policy.manifest-and-policy-cover-family] ✅ completed: 2026-04-25T19:37:10Z
  - Evidence: `docs/crate-extraction/policy.ncl`, `docs/crate-extraction.md`, `scripts/check-crate-extraction-readiness.rs`, `openspec/changes/extract-kv-branch-commit-dag/evidence/implementation-diff.txt`

- [x] I5 Add a downstream-style consumer fixture that imports `aspen-kv-branch` and `aspen-commit-dag` directly, exercises branch overlay and commit DAG APIs, and records metadata proving root Aspen and compatibility shells are absent. [covers=kv-branch-commit-dag-extraction.downstream-consumer-proof,kv-branch-commit-dag-extraction.downstream-consumer-proof.fixture-uses-canonical-crates] ✅ completed: 2026-04-25T19:37:10Z
  - Evidence: `openspec/changes/extract-kv-branch-commit-dag/fixtures/downstream-branch-dag/`, `openspec/changes/extract-kv-branch-commit-dag/evidence/i5-downstream-branch-dag-test.txt`, `openspec/changes/extract-kv-branch-commit-dag/evidence/i5-downstream-branch-dag-metadata.json`, `openspec/changes/extract-kv-branch-commit-dag/evidence/i5-downstream-branch-dag-forbidden-grep.txt`

- [x] V1 Save compile and dependency-boundary evidence for `cargo check -p aspen-commit-dag`, `cargo check -p aspen-kv-branch`, `cargo check -p aspen-kv-branch --no-default-features`, `cargo check -p aspen-kv-branch --features commit-dag`, and matching `cargo tree --edges normal` commands proving forbidden crates are absent. [covers=kv-branch-commit-dag-extraction.commit-dag-avoids-raft-compat.commit-dag-graph-excludes-app-shells,kv-branch-commit-dag-extraction.kv-branch-feature-topology.default-avoids-commit-dag,kv-branch-commit-dag-extraction.kv-branch-feature-topology.commit-dag-feature-is-reusable] ✅ completed: 2026-04-25T19:37:10Z
  - Evidence: `openspec/changes/extract-kv-branch-commit-dag/evidence/v1-cargo-check-aspen-commit-dag.txt`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v1-cargo-check-aspen-kv-branch-default.txt`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v1-cargo-check-aspen-kv-branch-no-default.txt`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v1-cargo-check-aspen-kv-branch-commit-dag.txt`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v1-cargo-tree-aspen-commit-dag.txt`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v1-cargo-tree-aspen-kv-branch-default.txt`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v1-cargo-tree-aspen-kv-branch-no-default.txt`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v1-cargo-tree-aspen-kv-branch-commit-dag.txt`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v1-commit-dag-forbidden-grep.txt`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v1-kv-branch-default-forbidden-grep.txt`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v1-kv-branch-commit-dag-forbidden-grep.txt`

- [x] V2 Run and save `cargo nextest run -p aspen-commit-dag` and `cargo nextest run -p aspen-kv-branch --features commit-dag`, including positive and negative tests for hash parsing, corruption detection, conflict detection, empty branch commits, and over-limit branch commits. [covers=kv-branch-commit-dag-extraction.commit-dag-avoids-raft-compat.hash-helpers-owned-by-family,kv-branch-commit-dag-extraction.kv-branch-feature-topology.commit-dag-feature-is-reusable] ✅ completed: 2026-04-25T19:37:10Z
  - Evidence: `openspec/changes/extract-kv-branch-commit-dag/evidence/v2-nextest-aspen-commit-dag.txt`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v2-nextest-aspen-kv-branch-commit-dag.txt`

- [x] V3 Save compatibility evidence for representative Aspen consumers: `aspen-jobs --features kv-branch`, `aspen-ci-executor-shell --features kv-branch`, `aspen-deploy --features kv-branch`, `aspen-fuse --features kv-branch`, `aspen-docs --features commit-dag-federation`, and `aspen-cli --features commit-dag`. [covers=kv-branch-commit-dag-extraction.downstream-consumer-proof.fixture-uses-canonical-crates] ✅ completed: 2026-04-25T19:37:10Z
  - Evidence: `openspec/changes/extract-kv-branch-commit-dag/evidence/v3-cargo-check-aspen-jobs-kv-branch.txt`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v3-cargo-check-aspen-ci-executor-shell-kv-branch.txt`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v3-cargo-check-aspen-deploy-kv-branch.txt`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v3-cargo-check-aspen-fuse-kv-branch.txt`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v3-cargo-check-aspen-docs-commit-dag-federation.txt`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v3-cargo-check-aspen-cli-commit-dag.txt`

- [x] V4 Run `scripts/check-crate-extraction-readiness.rs --candidate-family kv-branch-commit-dag` and save checker output plus any negative mutation checks proving forbidden dependency, missing owner, and invalid readiness state failures are caught. [covers=kv-branch-commit-dag-extraction.inventory-and-policy.manifest-and-policy-cover-family] ✅ completed: 2026-04-25T19:37:10Z
  - Evidence: `openspec/changes/extract-kv-branch-commit-dag/evidence/v4-readiness.md`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v4-readiness.json`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v4-negative-mutations-summary.txt`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v4-negative-forbidden-raft-summary.txt`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v4-negative-missing-owner-summary.txt`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v4-negative-invalid-readiness-summary.txt`, `openspec/changes/extract-kv-branch-commit-dag/evidence/v4-negative-missing-downstream-summary.txt`

## Verification Commands

### `openspec/changes/extract-kv-branch-commit-dag/evidence/run-verification.sh`

- Status: pass
- Artifact: `openspec/changes/extract-kv-branch-commit-dag/evidence/v1-cargo-check-aspen-commit-dag.txt`
- Artifact: `openspec/changes/extract-kv-branch-commit-dag/evidence/v2-nextest-aspen-commit-dag.txt`
- Artifact: `openspec/changes/extract-kv-branch-commit-dag/evidence/v3-cargo-check-aspen-cli-commit-dag.txt`

### `CARGO_TARGET_DIR=/tmp/aspen-kv-branch-fixture cargo test --manifest-path openspec/changes/extract-kv-branch-commit-dag/fixtures/downstream-branch-dag/Cargo.toml`

- Status: pass
- Artifact: `openspec/changes/extract-kv-branch-commit-dag/evidence/i5-downstream-branch-dag-test.txt`

### `scripts/check-crate-extraction-readiness.rs --policy docs/crate-extraction/policy.ncl --inventory docs/crate-extraction.md --manifest-dir docs/crate-extraction --candidate-family kv-branch-commit-dag --output-json openspec/changes/extract-kv-branch-commit-dag/evidence/v4-readiness.json --output-markdown openspec/changes/extract-kv-branch-commit-dag/evidence/v4-readiness.md`

- Status: pass
- Artifact: `openspec/changes/extract-kv-branch-commit-dag/evidence/v4-readiness.md`
- Artifact: `openspec/changes/extract-kv-branch-commit-dag/evidence/v4-readiness.json`

### `openspec/changes/extract-kv-branch-commit-dag/evidence/v4-run-negative-mutations.sh`

- Status: pass
- Artifact: `openspec/changes/extract-kv-branch-commit-dag/evidence/v4-negative-mutations-summary.txt`
- Artifact: `openspec/changes/extract-kv-branch-commit-dag/evidence/v4-negative-forbidden-raft-summary.txt`
- Artifact: `openspec/changes/extract-kv-branch-commit-dag/evidence/v4-negative-missing-owner-summary.txt`
- Artifact: `openspec/changes/extract-kv-branch-commit-dag/evidence/v4-negative-invalid-readiness-summary.txt`
- Artifact: `openspec/changes/extract-kv-branch-commit-dag/evidence/v4-negative-missing-downstream-summary.txt`

### `scripts/openspec-preflight.sh extract-kv-branch-commit-dag`

- Status: pass
- Artifact: `openspec/changes/extract-kv-branch-commit-dag/evidence/openspec-preflight.txt`
