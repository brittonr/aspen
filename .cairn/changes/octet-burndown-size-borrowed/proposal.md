# Proposal: Octet burn-down, sink parameters instead of borrowed `&mut Vec`

## Why

`borrowed_argument_types` is the second C3 size-shape slice. It has 360 workspace findings at 180 functions, and
every one is a `&mut Vec<T>` parameter; none is a `&String`, `&Vec<T>`, or `&PathBuf`. The base is C3a
(`105c68f24`), with 3052 workspace findings and 1287 lib findings. All 180 functions are private, so no public
signature changes. The family must reach zero by repair.

## What Changes

- Library helpers take the existing crate-private `crate::bounded::VecSink<T>` sink,
  `&mut impl crate::bounded::VecSink<T>`, which the bounded-growth helpers already use. Their bodies call
  `push_item`, `item_count`, `extend_cloned_items`, and a new `extend_items(impl IntoIterator<Item = T>)`.
- The two `molten` binary helpers cannot see the library-private trait. They take `&mut impl Extend<T>` and push with
  `extend([value])`.
- Three helpers change shape instead:
  - `visit_structural_value` owns its path stack. Its only caller built the stack and dropped it after the call.
  - `world_distribution::retention::normalize_refs` takes and returns the owned `Vec`, because sorting and
    deduplicating need the vector itself.
  - `validate_send_message` takes its diagnostics sink as a positional parameter next to its C3a input struct.
- In `molten-node-host`, the two push helpers `push_bounded_entry` and `push_bounded_name` are inlined. Each listing
  loop denies the next push with the `len() >= MAX_LOCAL_STORE_ENTRIES` guard that `unbounded_collection_growth`
  recognizes, and the denial message is byte-identical.

## Impact

- **Files**: 55 files, of which 51 are library files under `src/`, 2 are `molten` binary files, 1 is in
  `molten-node-host`, and 1 is `src/bounded/mod.rs`.
- **Public API**: none. Every changed function is private to its module or crate.
- **Testing**:
  - Pinned Octet root and lib runs.
  - fmt, and clippy `-D warnings`.
  - Full `cargo test --workspace`, including `fabricboundarycompat` and the `fabric_execution::` tests.
  - Harness and fabric-simulation byte identity against the base binary.

## Out of Scope

- No renames. `function_length` and `excessive_file_length` are the remaining C3 slices.
