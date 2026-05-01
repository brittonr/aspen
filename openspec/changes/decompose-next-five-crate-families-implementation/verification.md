# Verification Evidence

## Implementation Evidence

- Changed file: `docs/crate-extraction/foundational-types.md`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/foundational-types-compatibility.txt`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/foundational-types-downstream-metadata.json`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/foundational-types-forbidden-boundary.txt`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i5-aspen-traits-negative.txt`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i5-aspen-traits-positive.txt`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i5-foundational-readiness.json`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i5-foundational-readiness.md`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i5-foundational-readiness.stderr`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/fixtures/aspen-traits-capability-smoke/Cargo.lock`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/fixtures/aspen-traits-capability-smoke/Cargo.toml`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/fixtures/aspen-traits-capability-smoke/src/lib.rs`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/fixtures/aspen-traits-no-default-kvread-negative/Cargo.lock`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/fixtures/aspen-traits-no-default-kvread-negative/Cargo.toml`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/fixtures/aspen-traits-no-default-kvread-negative/src/lib.rs`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/fixtures/aspen-traits-no-default-types-smoke/Cargo.lock`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/fixtures/aspen-traits-no-default-types-smoke/Cargo.toml`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/fixtures/aspen-traits-no-default-types-smoke/src/lib.rs`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/tasks.md`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/verification.md`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i5-implementation-diff.patch`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/openspec-preflight-i5.txt`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/openspec-verify-i5.json`

## Task Coverage

- [x] I3 Extend `docs/crate-extraction/policy.ncl` and `scripts/check-crate-extraction-readiness.rs` so `foundational-types`, `auth-ticket`, `jobs-ci-core`, `trust-crypto-secrets`, and `testing-harness` are family-selectable, have policy entries, and enforce missing downstream fixture / missing compatibility evidence gates before readiness can be raised. [covers=architecture.modularity.next-decomposition-policy-covers-wave] ✅ completed: 2026-04-30T20:12:00Z
  - Evidence: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i3-v1-implementation-diff.patch`

- [x] V1 Save checker transcripts and negative mutation evidence proving at least forbidden runtime dependency, missing owner, invalid readiness state, missing downstream fixture, and missing compatibility evidence failures are caught for the selected-wave checker path. [covers=architecture.modularity.next-decomposition-policy-covers-wave] ✅ completed: 2026-04-30T20:12:00Z
  - Evidence: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-run-negative-mutations.sh`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-negative-mutations-summary.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-negative-missing-owner-summary.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-negative-invalid-readiness-summary.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-negative-forbidden-runtime-summary.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-negative-missing-downstream-summary.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-negative-missing-compatibility-summary.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-foundational-readiness.md`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-foundational-readiness.json`

- [x] I4 Move or gate `aspen-storage-types` Redb table-definition surface so reusable defaults keep portable storage types only, preserving shell-facing compatibility where needed. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof] ✅ completed: 2026-04-30T20:31:00Z
  - Evidence: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i4-storage-types-redb-boundary.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i4-storage-table-no-std-negative.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/foundational-types-downstream-metadata.json`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/foundational-types-forbidden-boundary.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/foundational-types-compatibility.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i4-foundational-readiness.md`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i4-foundational-readiness.json`

- [x] I5 Split/prove `aspen-traits` reusable KV capability traits and save downstream fixture, cargo tree, no-default/default, negative boundary, and representative consumer evidence. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof] ✅ completed: 2026-04-30T20:46:00Z
  - Evidence: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i5-aspen-traits-positive.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i5-aspen-traits-negative.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i5-foundational-readiness.md`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i5-foundational-readiness.json`, `openspec/changes/decompose-next-five-crate-families-implementation/fixtures/aspen-traits-capability-smoke/Cargo.toml`, `openspec/changes/decompose-next-five-crate-families-implementation/fixtures/aspen-traits-no-default-types-smoke/Cargo.toml`, `openspec/changes/decompose-next-five-crate-families-implementation/fixtures/aspen-traits-no-default-kvread-negative/Cargo.toml`

## Oracle Checkpoints

None.

## Review Scope Snapshot

### `git diff -- docs/crate-extraction/... scripts/check-crate-extraction-readiness.rs openspec/changes/decompose-next-five-crate-families-implementation`

- Status: captured
- Artifact: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i3-v1-implementation-diff.patch`

## Verification Commands

### `scripts/check-crate-extraction-readiness.rs --policy docs/crate-extraction/policy.ncl --inventory Cargo.toml --manifest-dir docs/crate-extraction --candidate-family foundational-types --output-json openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-foundational-readiness.json --output-markdown openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-foundational-readiness.md`

- Status: expected fail until family downstream/compatibility evidence is produced; confirmed selected-wave gates fire.
- Artifact: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-foundational-readiness.md`
- Artifact: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-foundational-readiness.json`

### `openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-run-negative-mutations.sh`

- Status: pass
- Artifact: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-negative-mutations-summary.txt`

### `scripts/openspec-preflight.sh decompose-next-five-crate-families-implementation`

- Status: pass
- Artifact: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/openspec-preflight-i3-v1.txt`


### `cargo check -p aspen-storage-types --no-default-features` and storage boundary checks

- Status: pass
- Artifact: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i4-storage-types-redb-boundary.txt`
- Negative artifact: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i4-storage-table-no-std-negative.txt`

### `scripts/check-crate-extraction-readiness.rs --policy docs/crate-extraction/policy.ncl --inventory Cargo.toml --manifest-dir docs/crate-extraction --candidate-family foundational-types --output-json .../i4-foundational-readiness.json --output-markdown .../i4-foundational-readiness.md`

- Status: pass
- Artifact: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i4-foundational-readiness.md`
- Artifact: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i4-foundational-readiness.json`


### `cargo check/test` for aspen-traits capability split

- Status: pass
- Artifact: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i5-aspen-traits-positive.txt`
- Negative artifact: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i5-aspen-traits-negative.txt`

### `scripts/check-crate-extraction-readiness.rs --policy docs/crate-extraction/policy.ncl --inventory Cargo.toml --manifest-dir docs/crate-extraction --candidate-family foundational-types --output-json .../i5-foundational-readiness.json --output-markdown .../i5-foundational-readiness.md`

- Status: pass
- Artifact: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i5-foundational-readiness.md`
- Artifact: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i5-foundational-readiness.json`
