## Why

Cluster tests produce canonical receipts, but the lifecycle should also prove that repeated runs with equivalent declared inputs are semantically stable. Without a drift gate, hidden ambient state, path ordering, generated identifiers, or rendered-output reliance can slip into cluster evidence.

## What Changes

- Add deterministic drift checks for cluster init/start/status/stop and selected VM child evidence workflows.
- Run equivalent workflows in fresh isolated state roots and compare canonical refs after explicit allowed-variance normalization.
- Add negative fixtures for undeclared path drift, runtime-path drift, ordering drift, ambient state, unstable map ordering, retry-only success, and rendered-output-only changes.
- Expose the drift check through a focused local command or Nix check suitable for release review.

## Impact

Cluster evidence becomes more reproducible and easier to review. Drift pass evidence is limited to declared deterministic inputs and does not cover live-only VM observations unless they have recorded replay logs.
