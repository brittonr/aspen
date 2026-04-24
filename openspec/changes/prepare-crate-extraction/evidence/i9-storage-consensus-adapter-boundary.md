# I9 Storage/consensus adapter boundary evidence

## Scope

Task I9 requires `aspen-raft-network` to remain the explicit iroh/IRPC adapter while storage plus reusable consensus/facade contracts compile without concrete iroh endpoint construction.

## Implementation delta

- `crates/aspen-redb-storage/Cargo.toml` now depends on leaf `aspen-constants` instead of default `aspen-core` for storage constants.
- `crates/aspen-redb-storage/src/lib.rs` re-exports the same storage constants from `aspen_constants::{api,raft}`.
- `crates/aspen-redb-storage/src/verified/kv.rs` doctest now uses `LeaseExpirationInput`, matching the functional-core API.

This keeps storage defaults on leaf constants, integrity helpers, and serialization only; concrete transport remains in `aspen-raft-network`.

## Baseline

- Command: `cargo test -p aspen-redb-storage`
- Status before this change: failed in doctest `LeaseExpirationInput` because the example still called `is_lease_expired(1000, 2000)` after the API had moved to `LeaseExpirationInput`.
- Transcript: `openspec/changes/prepare-crate-extraction/evidence/baseline-redb-storage-doctest-failure.txt`

## Positive verification

- Command: `cargo test -p aspen-redb-storage`
- Status: pass; 65 unit tests, 19 proptest/integration tests, and 9 runnable doctests passed; one ignored crate-root example remains ignored by design.
- Transcript: `openspec/changes/prepare-crate-extraction/evidence/aspen-redb-storage-tests.txt`

- Command: `cargo check -p aspen-redb-storage --no-default-features && cargo check -p aspen-raft-kv-types --no-default-features && cargo check -p aspen-raft-kv --no-default-features && cargo check -p aspen-raft-network --no-default-features`
- Status: pass after the storage dependency cleanup.
- Transcript: `openspec/changes/prepare-crate-extraction/evidence/i9-compile-matrix-after.txt`

## Dependency proof

- Command: `cargo tree -p aspen-redb-storage --no-default-features --edges normal`
- Output: `openspec/changes/prepare-crate-extraction/evidence/i9-aspen-redb-storage-tree-after.txt`
- Result: default storage tree contains only `aspen-constants`, `blake3`, `hex`, and `serde` direct dependencies; it no longer pulls `aspen-core`.

- Command: `cargo tree -p aspen-raft-kv-types --no-default-features --edges normal`
- Output: `openspec/changes/prepare-crate-extraction/evidence/i9-aspen-raft-kv-types-tree.txt`

- Command: `cargo tree -p aspen-raft-kv --no-default-features --edges normal`
- Output: `openspec/changes/prepare-crate-extraction/evidence/i9-aspen-raft-kv-tree.txt`

- Command: `rg -n "\b(iroh|iroh-base|irpc|aspen-raft-network|aspen-transport)\b" <storage/types/facade tree files>`
- Output: `openspec/changes/prepare-crate-extraction/evidence/i9-storage-consensus-no-iroh-grep.txt`
- Result: empty output; storage, reusable app types, and facade default graphs do not reach concrete iroh, IRPC, transport, or the adapter crate.

- Command: `rg -n "\b(aspen-core|aspen-core-shell|iroh|iroh-base|aspen-raft-network|aspen-transport)\b" openspec/changes/prepare-crate-extraction/evidence/i9-aspen-redb-storage-tree-after.txt`
- Output: `openspec/changes/prepare-crate-extraction/evidence/i9-aspen-redb-storage-forbidden-after.txt`
- Result: empty output; `aspen-redb-storage` default no longer reaches the app/core shell or concrete transport dependencies.

- Command: `cargo tree -p aspen-raft-network --no-default-features --edges normal`
- Output: `openspec/changes/prepare-crate-extraction/evidence/i9-aspen-raft-network-tree.txt`
- Result: adapter crate remains the explicit place where iroh/IRPC/transport dependencies appear.
