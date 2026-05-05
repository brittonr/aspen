## Why

Aspen already uses Nickel for CI configuration, node examples, deploy protocol contracts, and test-suite manifests, but the boundary is uneven: some schemas are hand-written, some are generated, and important operator evidence/config surfaces still depend on Rust-only validation or duplicated documentation. That makes drift likely exactly where Aspen needs fail-closed behavior: dogfood receipts, CI/deploy config, node profiles, feature bundles, trust/bootstrap policy, snix executor policy, and test/fault manifests.

## What Changes

- **Typed Nickel contract boundary**: define which Aspen surfaces must have exported Nickel contracts with explicit type/contract validation.
- **Rust-to-Nickel generation policy**: for schema-bearing Rust structs/enums that are serialized, persisted, or operator-facing, generate Nickel contracts from Rust-derived schema metadata rather than maintaining parallel hand-written schemas.
- **Nickel-authored configuration policy**: for human-authored modular config, keep Nickel as the source of truth and have Rust consume validated exported JSON/TOML values.
- **Drift gates**: add freshness checks proving generated Nickel contracts match Rust types and generated inventories match Nickel manifests.
- **Crunch prior art**: require the contract registry to classify reusable `../crunch/crunch` Nickel/Rust schema patterns as vendored, adapted, or rejected before Aspen implementation work.
- **Security boundary**: prevent Nickel configs/contracts from embedding secret material; validate secret references and capability handles instead.

## Scope

- **In scope**: dogfood/CI receipts, deploy protocol/config, CI pipeline config, node/cluster/profile config, feature bundles, test harness manifests, patchbay/network scenarios, trust/quorum/bootstrap policy, snix/build executor policy, crate-extraction policy, and schema generation/freshness checks.
- **Out of scope**: moving Raft logic, protocol discriminant ownership, token/capability cryptographic internals, hot-path runtime constants, or secret values into Nickel.

## Impact

- **Files**: `schemas/*.ncl`, `crates/aspen-ci/src/config/schema/*.ncl`, `crates/aspen-nickel/src/schema/*.ncl`, `test-harness/schema.ncl`, `docs/crate-extraction/policy.ncl`, generator/checker scripts, and Rust structs that derive exported schemas.
- **APIs**: new or extended schema-generation/checker entry points; Rust runtime parsing remains from validated exported data.
- **Testing**: strict OpenSpec validation, Nickel typecheck/export tests, generated-schema freshness checks, Rust round-trip tests, and negative fixtures for invalid config/evidence.
