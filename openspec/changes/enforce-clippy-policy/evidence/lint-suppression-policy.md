# Rollout-scope lint suppression policy

## Standard crate-root policy block

The rollout scope uses exactly this block in every participating crate root:

```rust
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![allow(clippy::module_name_repetitions)]
```

## Additional suppressions in rollout scope

- `aspen-layer`: no extra crate-root Clippy suppressions were added in this change.
- `aspen-time`: retains pre-existing Tiger Style crate-root allows for `unknown_lints`, `no_panic`, and named Tiger Style families. This change did **not** add new broad Clippy crate-root allows beyond the standardized `module_name_repetitions` baseline.

## Policy

Any future non-standard `#[allow(...)]` inside the rollout scope must be item-scoped by default. Broader module scope requires inline justification that item scope is impractical. Broad crate-wide Clippy escape hatches are deferred work and require explicit follow-up approval.
