# Design: Octet burn-down, sink parameters instead of borrowed `&mut Vec`

## Context

`borrowed_argument_types` flags parameters typed `&String`, `&Vec<T>`, `&mut Vec<T>`, or `&PathBuf` in free
functions and inherent methods outside test contexts. It does not flag trait-impl methods, and it asks for the most
general borrowed type unless ownership, capacity, or exact owned type identity is needed. All 180 sites are
`&mut Vec<T>` output parameters that only push, extend, or read the length. The one exception sorts and
deduplicates.

## Decisions

### Decision: Reuse `crate::bounded::VecSink`

**Choice:** Library sites take `&mut impl crate::bounded::VecSink<T>`. `VecSink` gains
`extend_items(impl IntoIterator<Item = T>)` for the six helpers that extend from an iterator.

**Rationale:** `VecSink` is the repository's established push-sink abstraction. The bounded-growth helpers
(`push_bounded`, `extend_bounded`, and `PushLimited`) already accept it. `item_count` keeps count-based bounds
available. Only `Vec` implements the trait, so each helper still compiles to one instantiation.

### Decision: Use std `Extend` in the binary crate

**Choice:** `collect_specs_under` and `push_optional_payload` take `&mut impl Extend<T>`.

**Rationale:** The trait is crate-private to the library, and these helpers only push. Making `VecSink` public would
add a public API item for two private binary helpers.

### Decision: Change ownership where the body needs the vector

**Choice:**
- `normalize_refs(Vec<String>) -> Vec<String>`. Callers use `std::mem::take` on the evidence field.
- `visit_structural_value` creates its own `["$"]` path stack.

**Rationale:** Sorting and deduplicating need the owned vector. The path stack's only caller did not read it after
the call, so the helper owning it matches the actual data flow.

### Decision: Inline the node-host push helpers with a recognized guard

**Choice:** Each listing loop checks `if collection.len() >= MAX_LOCAL_STORE_ENTRIES { return Err(entry_limit_error()); }`
before it pushes.

**Rationale:** The two helpers existed only to wrap a checked push. Inlining them with the direct guard keeps
`unbounded_collection_growth` at zero. The collection never exceeds the maximum, so the denied count is always
`MAX_LOCAL_STORE_ENTRIES + 1`, which is the value the previous `checked_add` message reported.
