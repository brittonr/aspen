## Why

The branch/DAG leaf graph and most representative consumers already have readiness evidence, but `aspen-docs --features commit-dag-federation` remains a concrete compatibility blocker caused by mixed iroh-blobs/iroh-docs and RNG API skew. Closing that blocker is the next durable step before the family can advance.

## What Changes

- Reproduce and record the docs feature failure.
- Isolate whether the mismatch is in feature selection, dependency version skew, or a narrow adapter API.
- Apply the smallest compatibility fix that keeps branch/DAG reusable defaults clean.
- Capture fresh passing docs feature evidence and rerun the readiness checker.

## Capabilities

### Modified Capabilities
- `kv-branch-commit-dag-extraction`: The docs federation representative consumer must compile before readiness can advance.

## Impact

- **Files**: likely `crates/aspen-docs`, feature wiring in Cargo manifests, and branch/DAG extraction evidence.
- **APIs**: No public branch/DAG API change expected unless required by compatibility evidence.
- **Dependencies**: Must not reintroduce `aspen-raft` or runtime shells into branch/DAG reusable graphs.
- **Testing**: `cargo check -p aspen-docs --features commit-dag-federation`, representative consumer checks, readiness checker.
