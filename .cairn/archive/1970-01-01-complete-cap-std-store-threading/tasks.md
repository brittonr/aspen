## Phase 1: Authority API

- [x] [serial] Inventory and classify every ambient filesystem operation in artifact, chunk, retention, dataspace, and exchange adapters; record bootstrap-shell, adversarial-test, and conversion-required classes. r[molten.chunk_store.cap_std_operational_roots]
- [x] [serial] Refactor the reusable capability root and pure relative-locator core so effectful APIs can borrow typed roots without recovering ambient paths. r[molten.chunk_store.cap_std_operational_roots] r[molten.chunk_store.cap_std_ambient_boundary]
- [x] [parallel] Add bounded, sorted capability-relative enumeration and fixed-leaf open helpers, including a handle bridge for Redb. r[molten.chunk_store.cap_std_relative_enumeration] r[molten.chunk_store.cap_std_backend_handles]

## Phase 2: Store cutover

- [x] [serial] Convert artifact and chunk reads, writes, listings, pin operations, synchronization, GC, and derived-index opens to typed capability roots. r[molten.chunk_store.cap_std_operational_roots] r[molten.chunk_store.cap_std_backend_handles]
- [x] [parallel] Convert retention store, bundle inventory, tombstone, receipt, and destructive mutation effects to capability-relative operations. r[molten.chunk_store.cap_std_operational_roots] r[molten.chunk_store.cap_std_relative_enumeration]
- [x] [parallel] Convert local dataspace and local exchange blob, envelope, ticket, publish, and fetch effects to typed capability roots. r[molten.chunk_store.cap_std_operational_roots] r[molten.chunk_store.cap_std_ambient_boundary]
- [x] [serial] Keep any path-taking compatibility surfaces as thin root-acquisition shells and remove obsolete ambient filesystem helper modules from converted adapters. r[molten.chunk_store.cap_std_ambient_boundary]

## Phase 3: Positive and negative verification

- [x] [parallel] Add positive tests for artifact, chunk, retention, dataspace, exchange, enumeration, and Redb workflows under declared roots. r[molten.chunk_store.cap_std_conversion_validation]
- [x] [parallel] Add negative tests for parent traversal, absolute and prefixed paths, remote/content locator confusion, symlinked intermediate and final components, non-regular entries, root replacement, and wrong-root handle substitution. r[molten.chunk_store.cap_std_conversion_validation]
- [x] [serial] Add scoped ast-grep positive and negative fixtures and a blocking adapter rule that rejects ambient filesystem reintroduction while permitting reviewed shell bootstrap and adversarial test setup. r[molten.chunk_store.cap_std_regression_gate]
- [x] [parallel] Update local-filesystem authority documentation to distinguish root acquisition, operational containment, compatibility shells, backend handles, and non-claims. r[molten.chunk_store.cap_std_ambient_boundary] r[molten.chunk_store.cap_std_backend_handles]

## Phase 4: Validation

- [x] [serial] Run focused artifact, chunk, retention, dataspace, exchange, Redb, and authority-audit tests, including every positive and negative boundary fixture. r[molten.chunk_store.cap_std_conversion_validation] r[molten.chunk_store.cap_std_regression_gate]
- [x] [serial] Run formatting, Clippy, Cairn validation, proposal/design/tasks gates, and the smallest relevant Nix checks before sync and archive. r[molten.chunk_store.cap_std_conversion_validation]

Validation evidence: `cargo fmt --all -- --check`, `cargo check --tests`, the full `cargo test` suite (1,136 tests), `cargo clippy --all-targets -- -D warnings`, positive/negative and converted-scope ast-grep scans, `nix build path:$PWD#checks.x86_64-linux.cap-std-store-authority`, and strict Cairn validation all passed on 2026-07-12.
