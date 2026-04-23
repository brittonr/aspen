# Rollout-scope unsafe inventory

## `aspen-layer`

`rg -n "unsafe" crates/aspen-layer/src` returned no matches in this change's rollout scope.

## `aspen-time`

`rg -n "unsafe" crates/aspen-time/src` returned no matches in this change's rollout scope.

Because the rollout scope contains no `unsafe` blocks, `clippy::undocumented_unsafe_blocks` remains enforced as deny-level policy with zero current exceptions.
