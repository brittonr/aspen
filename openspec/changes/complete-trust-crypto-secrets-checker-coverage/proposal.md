# Complete trust/crypto/secrets checker coverage

## Why

The trust/crypto/secrets owner/public API review kept the family at `workspace-internal` partly because readiness evidence still used an aggregate pseudo-candidate. That hid direct package dependency checks behind a warning instead of proving the canonical reusable crates.

## What Changes

- Map `trust-crypto-secrets` readiness checks to real reusable crate candidates: `aspen-crypto` and `aspen-secrets-core`.
- Split the policy metadata for those reusable surfaces while preserving the aggregate family status as `workspace-internal`.
- Refresh docs and evidence so remaining blockers are limited to `aspen-trust` async/default policy and serialization-contract evidence.

## Impact

- **Files**: readiness checker, crate-extraction policy, trust/crypto/secrets docs, OpenSpec evidence.
- **APIs**: no Rust public API changes.
- **Dependencies**: no dependency changes.
- **Testing**: checker run, direct package checks, dependency graphs, OpenSpec validation, Markdown lint, and `git diff --check`.
