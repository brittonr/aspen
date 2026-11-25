# Resource governance backpressure deltas

## ADDED Requirements

### Requirement: Retention planning and resource-limit backpressure admission live in no_std kernels
r[molten.resources.no_std_retention_and_resources_kernels] Retention/custody/pin planning (`molten-retention-core`) and resource-limit/backpressure admission (`molten-resources-core`) MUST live in `#![no_std]` + `alloc` crates atop `molten-preserves-core` without `PathBuf` or OS I/O dependencies.

#### Scenario: Retention GC plan and resource admission execute under no_std
- GIVEN a set of pinned refs, reachability facts, and resource hard-cap profiles
- WHEN `molten-retention-core` and `molten-resources-core` evaluate plans under `#![no_std]`
- THEN both MUST emit deterministic canonical plans and denials with zero filesystem or clock access.
