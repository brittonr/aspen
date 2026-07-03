# Design: Production profile domain contracts

## Context

`docs/production-node-profile.ncl` is the operator-facing source for production deployment profile JSON. Today it mostly verifies record shape. This change keeps Nickel as the static boundary and moves obvious domain validation into idempotent custom validators.

## Contract shape

Introduce pure Nickel contracts with `std.contract.from_validator` for scalar domains:

- `NonEmptyText` for profile names and reviewed marker strings.
- `Blake3Ref` for content refs of the form `blake3:<lowercase-hex-digest>`.
- `AbsolutePath` for `state_root`.
- `SafeRelativeDir` for state layout members such as ledger, Redb, chunks, identity, retention, and inbox directories.
- `PositiveInteger` for resource limits.

The validators remain immediate and idempotent. They do not normalize values, read files, or inspect the host system. Path contracts validate profile syntax only; existence, permissions, disk capacity, and migration status remain runtime/operator evidence.

## Export behavior

Valid profile values continue to export to the same JSON field names and scalar values. Invalid values fail at Nickel export, before any production readiness receipt is emitted. Downstream Rust loading still validates the exported JSON shape and remains a second fail-closed boundary rather than the only validator.

## Validation

The smallest implementation check is `nickel export docs/production-node-profile.ncl`. Follow-up fixture work adds positive and negative exports for each scalar domain.
