# Proposal: Octet burn-down, source files within the 300-line limit

## Why

`excessive_file_length` is the last C3 size-shape slice. The base is C3c (`52140a0bf`), which has 2499 workspace
findings and 1046 lib findings. `excessive_file_length` accounts for 548 of the workspace findings (305 sites) and 229
of the lib findings. The flagged set is:
- 303 Rust files: 200 existing `include!` part bodies and 103 plain module files.
- One symlinked alias, `tests/src/test/support.rs`, which is the same file as `src/test/support.rs`.
- `Cargo.lock`, which Octet counts because a protocol test embedded it with `include_str!`.

The family must reach zero by repair.

## What Changes

- Every flagged file moves into the repository's existing `include!` parts layout. Chunk text is the original text in
  its original order, cut at top-level item boundaries.
  - A plain module file `<dir>/<stem>.rs` keeps its inner attributes and outline `mod` declarations. It includes
    `<dir>/parts/<stem>/pNNN/body.rs` chunks at the positions they had in the file.
  - An over-long existing part keeps its first chunk. Its remaining chunks become new sibling part numbers, and the
    includer lists them in order.
  - An inline module that alone exceeds the limit keeps its attributes, `mod name {` line, and closing brace, and
    includes `<parts>/<name>/mNNN/pNNN/body.rs` chunks of its body. This follows the existing `tests/m000` convention.
  - Four inherent `impl` blocks that alone exceed the limit are re-emitted as consecutive `impl` blocks with the same
    header.
  - `src/lib.rs` keeps its crate attributes and all outline module declarations. Its other items, the
    `compat_module!` alias invocations, the prelude, the re-exports, and the smoke tests, move into one included part.
    rustfmt then sorts the now-contiguous module declarations.
- The protocol facade-boundary test reads `Cargo.lock` at run time instead of embedding it.
- Tooling references follow the moved code:
  - ast-grep rule `files:` lists and flake scan lists gain the new chunks of each listed file.
  - Flake literal checks on split hosts also scan the host's parts directory.
  - Tracey repair records, the inherited-debt classification, and the evidence matrix point at the chunk that
    carries each marker or test.
  - The repository Cairn policy evidence roots include the adoption module's parts directory, and the generated
    policy JSON is re-exported.

## Impact

- **Files**: 383 modified and 526 added. The concatenated source of every module is unchanged, apart from the
  duplicated `impl` headers, the `lib.rs` item order, and the `Cargo.lock` read.
- **Public API**: none. Module paths, item visibility, and item names are unchanged.
- **Testing**:
  - Pinned Octet root and lib runs.
  - Include-expansion comparison of every module host against the base tree.
  - fmt, and clippy `-D warnings`.
  - Full workspace tests, including `fabricboundarycompat` and the `fabric_execution::` tests.
  - Harness, fabric-simulation, and CLI-help byte identity against the base binary.
  - Touched-surface flake checks.

## Out of Scope

- No renames, and no module-tree restructuring beyond the parts layout. The five C2 names unmasked in C3c stay C2
  inputs.
