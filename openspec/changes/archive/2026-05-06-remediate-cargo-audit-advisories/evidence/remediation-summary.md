# Cargo audit remediation summary

Status: captured
Captured: 2026-05-06

## Remediated dependency clusters

- Wasmtime/plugin runtime: `wasmtime 36.0.3` -> `36.0.7`; removes 14 Wasmtime RustSec advisories from the lock graph.
- TLS/cert validation: `aws-lc-rs 1.16.1` -> `1.16.3`, `aws-lc-sys 0.38.0` -> `0.40.0`, `rustls-webpki 0.103.9` -> `0.103.13`; removes aws-lc and rustls-webpki vulnerability clusters.
- Archive extraction: `tar 0.4.44` -> `0.4.45`; removes tar archive advisories.
- Snix async tar exception: `astral-tokio-tar 0.5.6` remains through upstream `snix` revision `e20f82d...`; `flake.nix` now carries a bounded `RUSTSEC-2026-0066` ignore with exposure rationale and removal trigger.

## Remaining vulnerability delta

- Initial local audit: 23 vulnerabilities.
- After lock remediation before new bounded ignore: 15 vulnerabilities.
- Final local audit with explicit `RUSTSEC-2023-0071` and `RUSTSEC-2026-0066` ignores: 0 vulnerabilities.

## Regression repair

Focused `cargo check -p aspen-snix -p aspen-ci-executor-nix -p aspen-nix-cache-gateway --all-targets` exposed stale trait imports in `crates/aspen-snix/tests/migration_test.rs`. The test now imports `CacheLookup`, `CachePublish`, and `CacheStatsProvider`, and the focused migration tests pass.
