# R1 Baseline

Baseline protocol/wire compile, dependency, and compatibility-test evidence captured under `r1-baseline-logs/`.

Observed shape:
- `aspen-client-api` depends on portable `aspen-auth-core` through dependency key `aspen-auth` plus protocol crates.
- Forge/jobs/coordination protocol crates default to serialization-only dependencies.
- Existing `aspen-client-api` tests already pin postcard discriminants for client request/response variants.
