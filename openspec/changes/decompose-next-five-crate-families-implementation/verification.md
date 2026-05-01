# Verification Evidence

## Implementation Evidence

- Changed file: `docs/crate-extraction/trust-crypto-secrets.md`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i11-trust-crypto-secrets-inventory.txt`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i11-trust-crypto-secrets-surface-inventory.md`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i11-implementation-diff.patch`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/openspec-preflight-i11.txt`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/openspec-verify-i11.json`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/tasks.md`
- Changed file: `openspec/changes/decompose-next-five-crate-families-implementation/verification.md`

## Task Coverage

- [x] I3 Extend `docs/crate-extraction/policy.ncl` and `scripts/check-crate-extraction-readiness.rs` so `foundational-types`, `auth-ticket`, `jobs-ci-core`, `trust-crypto-secrets`, and `testing-harness` are family-selectable, have policy entries, and enforce missing downstream fixture / missing compatibility evidence gates before readiness can be raised. [covers=architecture.modularity.next-decomposition-policy-covers-wave] ✅ completed: 2026-04-30T20:12:00Z
  - Evidence: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i3-v1-implementation-diff.patch`

- [x] V1 Save checker transcripts and negative mutation evidence proving at least forbidden runtime dependency, missing owner, invalid readiness state, missing downstream fixture, and missing compatibility evidence failures are caught for the selected-wave checker path. [covers=architecture.modularity.next-decomposition-policy-covers-wave] ✅ completed: 2026-04-30T20:12:00Z
  - Evidence: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-run-negative-mutations.sh`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-negative-mutations-summary.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-negative-missing-owner-summary.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-negative-invalid-readiness-summary.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-negative-forbidden-runtime-summary.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-negative-missing-downstream-summary.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-negative-missing-compatibility-summary.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-foundational-readiness.md`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/v1-foundational-readiness.json`

- [x] I4 Move or gate `aspen-storage-types` Redb table-definition surface so reusable defaults keep portable storage types only, preserving shell-facing compatibility where needed. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof] ✅ completed: 2026-04-30T20:31:00Z
  - Evidence: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i4-storage-types-redb-boundary.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i4-storage-table-no-std-negative.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/foundational-types-downstream-metadata.json`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/foundational-types-forbidden-boundary.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/foundational-types-compatibility.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i4-foundational-readiness.md`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i4-foundational-readiness.json`

- [x] I5 Split/prove `aspen-traits` reusable KV capability traits and save downstream fixture, cargo tree, no-default/default, negative boundary, and representative consumer evidence. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof] ✅ completed: 2026-04-30T20:46:00Z
  - Evidence: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i5-aspen-traits-positive.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i5-aspen-traits-negative.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i5-foundational-readiness.md`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i5-foundational-readiness.json`, `openspec/changes/decompose-next-five-crate-families-implementation/fixtures/aspen-traits-capability-smoke/Cargo.toml`, `openspec/changes/decompose-next-five-crate-families-implementation/fixtures/aspen-traits-no-default-types-smoke/Cargo.toml`, `openspec/changes/decompose-next-five-crate-families-implementation/fixtures/aspen-traits-no-default-kvread-negative/Cargo.toml`

- [x] I6 Migrate portable consumers to canonical `aspen-auth-core` / `aspen-hooks-ticket` imports or document retained `aspen-auth` compatibility re-exports with owner, tests, and removal criteria. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof] ✅ completed: 2026-04-30T21:02:00Z
  - Evidence: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i6-client-api-auth-core-migration.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i6-client-api-auth-core-boundary.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i6-auth-ticket-readiness.md`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i6-auth-ticket-readiness.json`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/auth-ticket-downstream-metadata.json`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/auth-ticket-forbidden-boundary.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/auth-ticket-compatibility.txt`

- [x] I7 Add token/ticket serialization goldens, malformed-input negative tests, downstream fixture metadata, and compatibility evidence for auth/ticket consumers. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof] ✅ completed: 2026-04-30T21:36:00Z
  - Evidence: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i7-auth-ticket-serialization-clean.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/auth-ticket-downstream-metadata.json`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/auth-ticket-forbidden-boundary.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/auth-ticket-compatibility.txt`

- [x] I8 Identify reusable scheduler/config/run-state/artifact surfaces and gate worker/executor/runtime shells behind adapter crates or named features. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof] ✅ completed: 2026-04-30T22:05:00Z
  - Evidence: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i8-jobs-ci-core-inventory.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i8-jobs-ci-core-surface-inventory.md`

- [x] I9 Add downstream scheduler/config fixture metadata and negative boundary evidence rejecting root app, handler, process-spawn, shell, VM, and Nix executor leaks from reusable defaults. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof] ✅ completed: 2026-04-30T22:05:00Z
  - Evidence: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i9-jobs-ci-fixtures.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/jobs-ci-core-downstream-metadata.json`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/jobs-ci-core-forbidden-boundary.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/jobs-ci-core-compatibility.txt`

- [x] I10 Save compatibility checks for affected jobs/CI consumers, handlers, CLI/dogfood paths, and executor crates named in the manifest. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof] ✅ completed: 2026-04-30T22:22:00Z
  - Evidence: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i10-jobs-ci-compatibility.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/jobs-ci-core-compatibility.txt`

- [x] I11 Extract or gate pure Shamir/GF/HKDF/share-chain/reconfiguration state logic away from Raft/Iroh/secrets-service runtime shells. [covers=architecture.modularity.next-decomposition-standalone-and-compatibility-proof] ✅ completed: 2026-04-30T22:37:00Z
  - Evidence: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i11-trust-crypto-secrets-inventory.txt`, `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i11-trust-crypto-secrets-surface-inventory.md`

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


### `cargo check` for aspen-client-api auth-core migration

- Status: pass
- Artifact: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i6-client-api-auth-core-migration.txt`
- Boundary artifact: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i6-client-api-auth-core-boundary.txt`

### `scripts/check-crate-extraction-readiness.rs --policy docs/crate-extraction/policy.ncl --inventory Cargo.toml --manifest-dir docs/crate-extraction --candidate-family auth-ticket --output-json .../i6-auth-ticket-readiness.json --output-markdown .../i6-auth-ticket-readiness.md`

- Status: pass
- Artifact: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i6-auth-ticket-readiness.md`
- Artifact: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i6-auth-ticket-readiness.json`


### `cargo test/check` for auth-ticket serialization and boundary fixtures

- Status: pass
- Artifact: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i7-auth-ticket-serialization-clean.txt`
- Positive fixture: `openspec/changes/decompose-next-five-crate-families-implementation/fixtures/auth-ticket-portable-smoke/Cargo.toml`
- Negative fixture: `openspec/changes/decompose-next-five-crate-families-implementation/fixtures/auth-ticket-runtime-negative/Cargo.toml`


### `cargo check/tree` for jobs/CI core reusable surface inventory

- Status: pass
- Artifact: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i8-jobs-ci-core-inventory.txt`
- Inventory: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i8-jobs-ci-core-surface-inventory.md`

### `cargo test/check` for jobs/CI core portable and negative fixtures

- Status: pass
- Artifact: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i9-jobs-ci-fixtures.txt`
- Positive fixture: `openspec/changes/decompose-next-five-crate-families-implementation/fixtures/jobs-ci-core-portable-smoke/Cargo.toml`
- Negative fixture: `openspec/changes/decompose-next-five-crate-families-implementation/fixtures/jobs-ci-runtime-negative/Cargo.toml`


### `cargo check` for jobs/CI runtime compatibility

- Status: pass
- Artifact: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i10-jobs-ci-compatibility.txt`
- Packages checked: `aspen-ci-core`, `aspen-jobs-protocol`, `aspen-ci`, `aspen-jobs`, `aspen-ci-handler`, `aspen-job-handler`, `aspen-ci-executor-shell`, `aspen-ci-executor-vm`, `aspen-ci-executor-nix`


### `cargo check/tree` for trust/crypto/secrets reusable surface inventory

- Status: pass
- Artifact: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i11-trust-crypto-secrets-inventory.txt`
- Inventory: `openspec/changes/decompose-next-five-crate-families-implementation/evidence/i11-trust-crypto-secrets-surface-inventory.md`

## I11 trust/crypto/secrets inventory and pure-boundary gate

Task: I11 Extract or gate pure Shamir/GF/HKDF/share-chain/reconfiguration state logic away from Raft/Iroh/secrets-service runtime shells.

Changed files:

- docs/crate-extraction/trust-crypto-secrets.md
- openspec/changes/decompose-next-five-crate-families-implementation/evidence/i11-trust-crypto-secrets-inventory.txt
- openspec/changes/decompose-next-five-crate-families-implementation/evidence/i11-trust-crypto-secrets-surface-inventory.md
- openspec/changes/decompose-next-five-crate-families-implementation/evidence/i11-implementation-diff.patch
- openspec/changes/decompose-next-five-crate-families-implementation/evidence/openspec-preflight-i11.txt
- openspec/changes/decompose-next-five-crate-families-implementation/evidence/openspec-verify-i11.json
- openspec/changes/decompose-next-five-crate-families-implementation/tasks.md
- openspec/changes/decompose-next-five-crate-families-implementation/verification.md

Durable evidence:

- `evidence/i11-trust-crypto-secrets-inventory.txt`: successful cargo checks and dependency trees for `aspen-trust`, `aspen-secrets --no-default-features`, and `aspen-crypto`.
- `evidence/i11-trust-crypto-secrets-surface-inventory.md`: boundary decision naming `aspen-trust` as the reusable pure trust surface, `aspen-secrets --no-default-features` as feature-gated runtime separation, and current `aspen-crypto` Iroh/tokio filesystem helpers as a boundary risk for the next slice.
- `evidence/i11-implementation-diff.patch`: source/docs/evidence diff for this slice.
- `evidence/openspec-preflight-i11.txt`: OpenSpec preflight output for this slice.
- `evidence/openspec-verify-i11.json`: OpenSpec validation output for this slice.

Result: I11 establishes the reusable trust/crypto/secrets boundary without moving code yet: pure Shamir/GF256/HKDF/share-chain/envelope/reconfiguration logic is centered in `aspen-trust`, runtime secrets-service/handler/SOPS/client transport remains outside reusable defaults, and `aspen-crypto` needs follow-up narrowing before it can be treated as wholly transport-free.
