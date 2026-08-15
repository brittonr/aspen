## Why

The existing deterministic drift comparator can compare canonical artifacts, but full cluster lifecycle reproducibility needs a stable summary that captures manifest refs, per-node receipt refs, decisions, ordering, and declared variance. Without that summary, drift checks either compare too little or rely on rendered output and paths.

## What Changes

- Add a cluster lifecycle summary model designed for drift comparison.
- Feed two fresh lifecycle runs into the drift comparator using canonical refs and explicit non-semantic variance declarations.
- Add negative fixtures for changed child refs, node ordering drift, undeclared runtime paths, ambient state, retry-only success, and rendered-output-only stability.
- Expose the focused check through a local command or Nix check suitable for release review.

## Impact

Cluster lifecycle reproducibility becomes a reviewable gate instead of an ad hoc rerun. Drift evidence remains deterministic local evidence only and does not cover live-only VM observations unless recorded replay logs are supplied.
