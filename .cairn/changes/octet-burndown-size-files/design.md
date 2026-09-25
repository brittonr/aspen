# Design: Octet burn-down, source files within the 300-line limit

## Context

`excessive_file_length` flags every source file in the compiler's source map that is longer than 300 lines. This
includes `include!` part bodies and files embedded with `include_str!`. The repository already splits large modules
into `parts/<stem>/pNNN/body.rs` bodies included in order, and nests inline test modules under `<name>/mNNN/`.
Since the earlier splits, 200 of those part bodies have grown past the limit, and 103 plain module files never used
the layout.

## Decisions

### Decision: Split mechanically at top-level item boundaries, verbatim

**Choice:** A scanner finds depth-0 item boundaries. It skips comments, string, raw-string, byte-string, and char
literals, and tells char literals apart from lifetimes. Each item carries the blank and comment lines before it, so
doc comments, attributes, and traceability markers stay with their item. Items are packed greedily into chunks of at
most 290 lines, in order.

**Rationale:** Moving text verbatim keeps every token, marker, and ordering, including `macro_rules!` textual
scope. Outline `mod` declarations stay in their host at their original position, so module path resolution and
macro visibility are unchanged. Inner attributes and inner doc comments stay in the host because included files
cannot carry them. `expandparts.py` inlines every include in the base tree and the candidate tree and compares the
results.

### Decision: Append new part numbers instead of renumbering

**Choice:** An over-long part keeps its path for its first chunk. The rest get the next unused sibling numbers, and
the includer lists them directly after the original line.

**Rationale:** Untouched parts keep their paths. Tracey evidence, ast-grep rules, flake scan lists, and docs that
name them stay valid. Only references to split files need re-pointing.

### Decision: Split oversize inline modules and inherent impls structurally

**Choice:**
- Inline modules longer than one chunk keep their wrapper in place and include nested parts, which is the existing
  `tests/m000` convention.
- Oversize inherent `impl` blocks repeat their header, generics, and where clause for each continuation block. No
  trait impl needed splitting.

**Rationale:** Rust lets an inherent impl be split across blocks without changing semantics, and each method keeps
its visibility, attributes, and order.

### Decision: Hoist non-module items out of `lib.rs`

**Choice:** `src/lib.rs` keeps the crate attributes and all outline module declarations. The alias invocations, the
`compat_module!` definition, the prelude, the re-exports, and the smoke tests move into `src/parts/lib/p000/body.rs`,
which is included last. rustfmt then sorts the contiguous module declarations.

**Rationale:** Outline module declarations in included files would resolve their paths relative to the part. Order
among items is not significant here:
- No child module uses `compat_module!`.
- There is no `#[macro_use]`.
- The line multiset of the expanded crate root is identical.

### Decision: Read `Cargo.lock` at run time in the facade-boundary test

**Choice:** `facade_dependency_boundary_rejects_chorus_import_drift` reads `concat!(env!("CARGO_MANIFEST_DIR"),
"/Cargo.lock")` with `std::fs::read_to_string` and keeps `include_str!` for `Cargo.toml`.

**Rationale:** A lockfile cannot be split, and embedding it puts a 9822-line file into the source map. The test still
checks the same bytes.

### Decision: Re-point tooling at the moved code

**Choice:**
- Every list that names a split file (ast-grep `files:` lists and flake scan arguments) gains that file's new chunks.
- Flake literal checks on split hosts also scan the host's parts directory.
- Tracey repair records and the inherited-debt classification point at the chunk that carries each `impl` or
  `verify` marker, in both the Nickel source and the generated JSON.
- Evidence-matrix targets point at the chunk that defines each named test.
- The repository Cairn policy adds `src/parts/live_binding_adoption` as an evidence root, and the generated JSON is
  re-exported with `nickel export`.

**Rationale:** Without this, each check would silently scan only the host's include lines. It would pass without
checking the moved code.
