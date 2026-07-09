## Why

Molten's architecture separates evidence, policy, runtime, and adapters, but the current single-crate shape lets these concerns depend on each other informally. Evidence construction, policy admission, runtime execution, and adapter IO need explicit ownership before large-scale crate extraction.

## What Changes

- Define intended compile-time ownership for evidence, policy, runtime, and adapters.
- Separate evidence value construction and verification from policy admission decisions.
- Keep runtime cores consuming admitted policy/evidence inputs rather than reaching into adapter shells.
- Preserve existing receipt and admission semantics while documenting staged extraction boundaries.

## Impact

This change prepares the codebase for a multi-crate architecture without changing canonical artifacts or runtime trust claims. It reduces circular coupling risk between evidence, policy, runtime, and adapter layers.
