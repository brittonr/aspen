## Phase 1: Pure plan and capability shell

- [x] [serial] Add a pure materialization member/path model, policy-driven bounds, duplicate and reserved-name checks, replacement decisions, deterministic ordering, and BLAKE3 plan identity. r[molten.filesystem_materialization.plan]
- [x] [serial] Add a `MaterializationRoot` shell for capability-relative directories, files, readback, staged publication, and cleanup without exposing ambient descendant paths. r[molten.filesystem_materialization.root]
- [x] [parallel] Add source-directory read capabilities and bounded streaming content-ref verification for admitted members. r[molten.filesystem_materialization.root] r[molten.filesystem_materialization.receipt]

## Phase 2: Existing surface migration

- [x] [parallel] Migrate repro export and unpack fixed-member workflows to pure plans and the capability-rooted writer. r[molten.filesystem_materialization.root] r[molten.filesystem_materialization.commit]
- [x] [parallel] Migrate retention candidate bundle output, artifact-group inventory, profiles, and verification scans to logical entries and capability-relative reads/writes. r[molten.filesystem_materialization.root] r[molten.filesystem_materialization.archive_members]
- [x] [serial] Migrate dogfood release evidence directories and tar archive source/destination files to explicit read/write capabilities while preserving release member semantics. r[molten.filesystem_materialization.archive_members] r[molten.filesystem_materialization.receipt]

## Phase 3: Commit and archive hardening

- [x] [serial] Implement in-root staging keyed by plan identity, explicit replacement policy, verified publication, and cleanup or quarantine without minting pass receipts for partial results. r[molten.filesystem_materialization.commit]
- [x] [parallel] Apply the canonical archive-member policy to tar read/write paths and reject links, special entries, traversal, prefixes, separator ambiguity, normalized duplicates, and over-bound members. r[molten.filesystem_materialization.archive_members]
- [x] [parallel] Emit portable materialization receipts that bind logical members, refs, bounds, plan identity, decision, diagnostics, and non-claims while treating host paths as display-only. r[molten.filesystem_materialization.receipt]

## Phase 4: Positive, negative, and structural tests

- [x] [parallel] Add positive repro, retention, release-directory, and archive round-trip fixtures under clean and pre-existing admitted destinations. r[molten.filesystem_materialization.validation]
- [x] [parallel] Add negative traversal, absolute/prefixed path, duplicate, symlink parent/leaf, wrong-root, special archive entry, stale plan, tampered source, partial write, and replacement-policy fixtures. r[molten.filesystem_materialization.validation]
- [x] [serial] Add scoped ast-grep fixtures and a blocking rule for ambient descendant writes and generic archive unpack in converted materializers. r[molten.filesystem_materialization.regression_gate]
- [x] [parallel] Document materialization authority, staging, archive policy, portable receipts, bounds, and non-claims. r[molten.filesystem_materialization.receipt]

## Phase 5: Validation

- [x] [serial] Run focused materialization core, repro, retention bundle, release export, archive, and structural-authority positive and negative tests. r[molten.filesystem_materialization.validation] r[molten.filesystem_materialization.regression_gate]
- [x] [serial] Run formatting, Clippy, Cairn validation, proposal/design/tasks gates, and relevant Nix checks before sync and archive. r[molten.filesystem_materialization.validation]

Validation evidence: `cargo fmt --all -- --check`, the full `cargo test` suite (1,176 tests), `cargo clippy --all-targets -- -D warnings`, focused materialization/repro/retention/release archive positive and negative tests, positive/negative and converted-scope ast-grep scans, `nix build path:$PWD#checks.x86_64-linux.materialization-authority --no-link`, and Cairn proposal/design/tasks gates plus strict validation passed on 2026-07-12.
