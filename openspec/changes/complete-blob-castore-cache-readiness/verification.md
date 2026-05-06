# Verification: complete-blob-castore-cache-readiness

## Implementation Evidence

- Changed file: `Cargo.lock`
- Changed file: `Cargo.toml`
- Changed file: `crates/aspen-blob/Cargo.toml`
- Changed file: `vendor/iroh-blobs/src/api/remote.rs`
- Changed file: `docs/crate-extraction.md`
- Changed file: `docs/crate-extraction/blob-castore-cache.md`
- Changed file: `docs/crate-extraction/policy.ncl`
- Changed file: `scripts/aspen-core-no-std-transitives.txt`
- Changed file: `docs/dependency-reviews/aspen-core-no-std/deps-transitive-review-blake3.md`
- Changed file: `docs/dependency-reviews/aspen-core-no-std/deps-transitive-review-cpufeatures.md`
- Changed file: `openspec/changes/complete-blob-castore-cache-readiness/tasks.md`
- Changed file: `openspec/changes/complete-blob-castore-cache-readiness/verification.md`
- Changed file: `openspec/changes/complete-blob-castore-cache-readiness/fixtures/downstream-blob/Cargo.toml`
- Changed file: `openspec/changes/complete-blob-castore-cache-readiness/fixtures/downstream-blob/src/lib.rs`
- Changed file: `openspec/changes/complete-blob-castore-cache-readiness/fixtures/downstream-blob/Cargo.lock`
- Changed file: `openspec/changes/complete-blob-castore-cache-readiness/fixtures/downstream-cache-castore/Cargo.toml`
- Changed file: `openspec/changes/complete-blob-castore-cache-readiness/fixtures/downstream-cache-castore/src/lib.rs`
- Changed file: `openspec/changes/complete-blob-castore-cache-readiness/fixtures/downstream-cache-castore/Cargo.lock`
- Changed file: `openspec/changes/complete-blob-castore-cache-readiness/evidence/i6-package-checks.txt`
- Changed file: `openspec/changes/complete-blob-castore-cache-readiness/evidence/i6-downstream-fixture-checks.txt`
- Changed file: `openspec/changes/complete-blob-castore-cache-readiness/evidence/blob-castore-cache-readiness.md`
- Changed file: `openspec/changes/complete-blob-castore-cache-readiness/evidence/closeout-verification.txt`

## Task Coverage

- [x] Create proposal, design, delta spec, and task rail for `complete-blob-castore-cache-readiness`.
  - Evidence: `openspec/changes/complete-blob-castore-cache-readiness/proposal.md`, `openspec/changes/complete-blob-castore-cache-readiness/design.md`, `openspec/changes/complete-blob-castore-cache-readiness/specs/blob-castore-cache-extraction/spec.md`, `openspec/changes/complete-blob-castore-cache-readiness/tasks.md`.
- [x] Audit current blob/castore/cache manifests and identify remaining workspace-internal reasons.
  - Evidence: `docs/crate-extraction/blob-castore-cache.md`, `docs/crate-extraction/policy.ncl`, `openspec/changes/complete-blob-castore-cache-readiness/evidence/i6-package-checks.txt`, `openspec/changes/complete-blob-castore-cache-readiness/evidence/i6-noq-conflict-tree.txt`, `openspec/changes/complete-blob-castore-cache-readiness/evidence/i6-blake3-digest-conflict-tree.txt`, `openspec/changes/complete-blob-castore-cache-readiness/evidence/i6-castore-irpc014-check.txt`.
- [x] Add/update downstream positive fixtures, negative policy fixtures, and readiness checker mapping for the family.
  - Evidence: `openspec/changes/complete-blob-castore-cache-readiness/fixtures/downstream-blob/Cargo.toml`, `openspec/changes/complete-blob-castore-cache-readiness/fixtures/downstream-blob/src/lib.rs`, `openspec/changes/complete-blob-castore-cache-readiness/fixtures/downstream-cache-castore/Cargo.toml`, `openspec/changes/complete-blob-castore-cache-readiness/fixtures/downstream-cache-castore/src/lib.rs`, `openspec/changes/complete-blob-castore-cache-readiness/evidence/i6-downstream-blob-metadata.json`, `openspec/changes/complete-blob-castore-cache-readiness/evidence/i6-downstream-cache-castore-metadata.json`, `openspec/changes/complete-blob-castore-cache-readiness/evidence/i6-downstream-blob-forbidden-grep.txt`, `openspec/changes/complete-blob-castore-cache-readiness/evidence/i6-downstream-cache-castore-forbidden-grep.txt`.
- [x] Run fixture builds, metadata capture, negative mutation checks, representative Aspen consumers, and update readiness docs.
  - Evidence: `openspec/changes/complete-blob-castore-cache-readiness/evidence/i6-downstream-fixture-checks.txt`, `openspec/changes/complete-blob-castore-cache-readiness/evidence/i6-castore-circuit-breaker-check.txt`, `docs/crate-extraction.md`, `docs/crate-extraction/blob-castore-cache.md`.
- [x] Run `openspec validate {name} --strict`, helper verification, repo-specific checks, and `git diff --check`.
  - Evidence: `openspec/changes/complete-blob-castore-cache-readiness/evidence/closeout-verification.txt`.
- [x] Sync/archive only after every implementation/evidence task is complete.
  - Evidence: `openspec/changes/complete-blob-castore-cache-readiness/evidence/closeout-verification.txt`.

## Verification Commands

- Command: `cargo test --manifest-path openspec/changes/complete-blob-castore-cache-readiness/fixtures/downstream-blob/Cargo.toml`
- Artifact: `openspec/changes/complete-blob-castore-cache-readiness/evidence/i6-downstream-fixture-checks.txt`
- Command: `cargo test --manifest-path openspec/changes/complete-blob-castore-cache-readiness/fixtures/downstream-cache-castore/Cargo.toml`
- Artifact: `openspec/changes/complete-blob-castore-cache-readiness/evidence/i6-downstream-fixture-checks.txt`
- Command: `cargo check -p aspen-blob --no-default-features`
- Artifact: `openspec/changes/complete-blob-castore-cache-readiness/evidence/i6-package-checks.txt`
- Command: `cargo check -p aspen-blob --features replication`
- Artifact: `openspec/changes/complete-blob-castore-cache-readiness/evidence/i6-package-checks.txt`
- Command: `cargo test -p aspen-castore circuit_breaker`
- Artifact: `openspec/changes/complete-blob-castore-cache-readiness/evidence/i6-castore-circuit-breaker-check.txt`
- Command: `cargo check -p aspen-castore --no-default-features`
- Artifact: `openspec/changes/complete-blob-castore-cache-readiness/evidence/i6-package-checks.txt`
- Command: `cargo test -p aspen-cache --no-default-features`
- Artifact: `openspec/changes/complete-blob-castore-cache-readiness/evidence/i6-package-checks.txt`
- Command: `cargo check -p aspen-cache --no-default-features`
- Artifact: `openspec/changes/complete-blob-castore-cache-readiness/evidence/i6-package-checks.txt`
- Command: `cargo check -p aspen-cache --features kv-index`
- Artifact: `openspec/changes/complete-blob-castore-cache-readiness/evidence/i6-package-checks.txt`
- Command: `python scripts/check-aspen-core-no-std-boundary.py --manifest-path crates/aspen-core/Cargo.toml --allowlist scripts/aspen-core-no-std-transitives.txt --output /tmp/aspen-core-no-std-current.txt --diff-output /tmp/aspen-core-no-std-diff.txt`
- Artifact: `openspec/changes/complete-blob-castore-cache-readiness/evidence/closeout-verification.txt`
- Command: `scripts/check-crate-extraction-readiness.rs --policy docs/crate-extraction/policy.ncl --inventory docs/crate-extraction.md --manifest-dir docs/crate-extraction --candidate-family blob-castore-cache --output-json openspec/changes/complete-blob-castore-cache-readiness/evidence/blob-castore-cache-readiness.json --output-markdown openspec/changes/complete-blob-castore-cache-readiness/evidence/blob-castore-cache-readiness.md`
- Artifact: `openspec/changes/complete-blob-castore-cache-readiness/evidence/blob-castore-cache-readiness.md`
- Command: `openspec validate complete-blob-castore-cache-readiness --strict`
- Artifact: `openspec/changes/complete-blob-castore-cache-readiness/evidence/closeout-verification.txt`
- Command: `scripts/openspec-preflight.sh complete-blob-castore-cache-readiness`
- Artifact: `openspec/changes/complete-blob-castore-cache-readiness/evidence/closeout-verification.txt`
- Command: `git diff --check`
- Artifact: `openspec/changes/complete-blob-castore-cache-readiness/evidence/closeout-verification.txt`
