## Phase 1: Apply request proofs

- [x] [serial] Inspect `apply_request_spec.rs` markers and document exact preconditions needed for version increment and batch last-applied facts.
- [x] [serial] Remove or narrow `set_increments_version` and `batch_updates_last_applied` trusted bodies with verified arithmetic proof bodies.
- [x] [depends:apply-proof] Run the Raft Verus root and focused apply/KV tests.

## Phase 2: Batcher proofs

- [x] [serial] Inspect `batcher_add_spec.rs` and `batcher_flush_spec.rs` byte-accounting/contiguity markers.
- [x] [parallel] Close `add_increases_bytes`, `add_preserves_bytes_consistency`, and `delete_add_bytes` with explicit byte-count arithmetic/preconditions.
- [x] [parallel] Close `ordered_batch_is_contiguous` using an explicit contiguous-pending precondition because the base modeled invariant only proves ordering/admission, not arbitrary gap-freedom.
- [x] [depends:batcher-proof] Run the Raft Verus root and focused write-batcher tests.

## Phase 3: Completion

- [x] [serial] Confirm no non-crypto `external_body` markers remain in Raft apply/batcher specs.
- [x] [serial] Sync/archive this OpenSpec after the Raft operational proof markers are removed or explicitly narrowed.
