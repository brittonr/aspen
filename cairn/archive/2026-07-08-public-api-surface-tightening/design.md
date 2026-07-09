## Context

`src/lib.rs` currently maps many implementation paths to public modules and compatibility aliases. That has helped preserve call sites through earlier renames, but it also makes it difficult to know which paths are stable API and which are migration scaffolding.

## Design

### API classification

Each public module or re-export should be classified as one of:

- stable API: intended for external or long-lived internal consumers;
- compatibility alias: preserved for migration but not the preferred path;
- internal implementation: should become `pub(crate)` or private;
- generated/test support: public only for tests or fixture scaffolding.

### Stable prelude or API module

The repository may introduce a small `api` or `prelude` module for intentionally stable types and functions. This should point consumers away from compatibility aliases and implementation details.

### Compatibility migration

Initial changes should avoid breaking public paths. Deprecation notes, documentation, or traceability can identify preferred replacements. Removal requires a separate change with compatibility evidence.

### Validation

API tightening should compile existing tests and include negative checks where feasible, such as compile-fail UI tests for internal-only imports or boundary checks that detect newly public modules lacking classification.

## Non-goals

- Do not perform a broad public API removal in the first slice.
- Do not hide types required for canonical artifact parsing or existing tests without a migration path.
- Do not make compatibility aliases the preferred long-term API.
