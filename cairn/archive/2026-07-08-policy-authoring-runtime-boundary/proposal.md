## Why

Molten uses Nickel and Cairn policy for reviewed authoring-time contracts, while runtime code should consume checked exports and canonical evidence. The current Aspen Cairn policy also lags the current Cairn CLI schema, showing the need for a clearer policy freshness and runtime-consumption boundary.

## What Changes

- Separate policy authoring, generated policy export, runtime policy consumption, and policy freshness validation.
- Ensure runtime admission does not invoke live Nickel or Cairn policy tooling as authority.
- Add freshness checks for generated Cairn/Nickel policy artifacts.
- Add positive and negative tests for valid exports, stale generated policy, and runtime boundary violations.

## Impact

Policy remains reviewable and typed at authoring time, while runtime behavior stays deterministic over checked artifacts and evidence refs.
