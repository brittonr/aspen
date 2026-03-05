# Comprehensive Test Coverage

## What

Add unit tests, integration tests, and NixOS VM tests across 5 recently-built subsystems that have significant coverage gaps:

1. **aspen-crypto** — 16 existing tests, missing edge cases for HMAC derivation, gossip topic, identity round-trips
2. **Ephemeral Pub/Sub** — Good broker/wire unit tests (25 total), but no VM test and TODOs for EventStream + CLI tests
3. **SOPS Operations** — 5 modules totaling 837 lines with **zero** unit tests: decrypt.rs, encrypt.rs, edit.rs, rotate.rs, updatekeys.rs
4. **FUSE Cache + Metadata** — 2 modules (cache.rs: 379 lines, metadata.rs: 169 lines) with `#[cfg(test)]` markers but 0 tests; no FUSE VM test
5. **Snapshot Recovery** — TODO for snapshot-under-failure tests in router_snapshot_t10_build_snapshot.rs

## Why

- **SOPS operations** are security-critical code with no test coverage — encrypt/decrypt/rotate/edit are the user-facing entry points
- **FUSE cache + metadata** have test scaffolding but no actual tests — cache invalidation bugs would cause stale reads
- **Pub/Sub** lacks a VM test proving multi-node ephemeral delivery over real iroh connections
- **FUSE** has no VM test verifying actual mount+read+write+delete through the filesystem
- All 6,308 existing tests pass — this builds on a stable base

## Scope

- ~30 new unit tests across aspen-crypto, aspen-secrets/sops, aspen-fuse
- ~5 new integration tests for pub/sub EventStream and SOPS round-trips
- 2 new NixOS VM tests: ephemeral-pubsub and fuse-operations
- Address all 5 existing TODOs in test files
