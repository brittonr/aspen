# Tasks: Canonical Content-Ref Helper Boundaries

## Phase 1: Shared construction

- [x] [serial] r[molten.runtime_spine.canonical_content_refs.helper_construction] Specify canonical helper construction for byte, hash, and hex inputs.
- [x] [serial] r[molten.runtime_spine.canonical_content_refs.filename_readback] Specify fail-closed filename/readback conversion through validated hex helpers.

## Phase 2: Alias and trust boundaries

- [x] [parallel] r[molten.runtime_spine.canonical_content_refs.scoped_aliases] Specify that transitional aliases such as `b3:` remain scoped evidence aliases and are not runtime content refs.
- [x] [parallel] r[molten.runtime_spine.canonical_content_refs.no_ad_hoc_formatting] Specify that subsystems must not hand-build canonical `blake3:` refs outside shared helpers.

## Phase 3: Validation evidence

- [x] [serial] r[molten.runtime_spine.canonical_content_refs.cleanup_tests] Validate focused content-ref, ledger/chunk/readback, remote dataspace, Iroh exchange, clippy, full test, and Octet clean evidence after cleanup.
