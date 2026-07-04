# Tasks: shared-preserves-content-ref

## Phase 1: Shared type

- [x] [serial] r[molten.preserves_content_ref.shared_newtype] Extend `preserves_rail::ContentRef` with serde, display/as-ref, ordering, and ergonomic conversion helpers needed by runtime DTOs.
- [x] [parallel] r[molten.preserves_content_ref.invalid_denials] Add positive tests for valid refs and negative tests for invalid prefix, uppercase hex, wrong length, non-hex input, empty input, and path-like strings.

## Phase 2: Migration

- [x] [serial] r[molten.preserves_content_ref.runtime_envelope] Replace runtime envelope's duplicate `ContentRef` with the shared type while preserving JSON and Preserves output.
- [x] [parallel] r[molten.preserves_content_ref.dto_migration] Migrate selected artifact, typed-storage, job, eval-cache, catalog, and schema DTO fields from raw `String` refs to the shared type.
- [x] [parallel] r[molten.preserves_content_ref.dto_migration] Keep compatibility constructors or accessors where public APIs still need strings.

## Phase 3: Validation

- [x] [serial] r[molten.preserves_content_ref.shared_newtype] r[molten.preserves_content_ref.runtime_envelope] Prove canonical envelope and artifact fixture hashes remain stable.
- [x] [serial] r[molten.preserves_content_ref.invalid_denials] Run focused Preserves, runtime, artifact, typed-storage, job, eval-cache, catalog, and schema tests.
