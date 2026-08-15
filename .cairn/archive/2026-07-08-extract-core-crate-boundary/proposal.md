## Why

Molten's architecture document describes a pure core layer, but the current workspace has one real crate and many domains compiled together. Without a compile-time core boundary, pure identity, envelope, bounded, and validation code can accidentally grow dependencies on adapters, CLI, filesystem, network, or runtime shells.

## What Changes

- Establish an `aspen-core` or `molten-core` workspace member for pure foundational types and deterministic validation.
- Move low-dependency core surfaces first: errors, bounded helpers, stable ids, content refs, envelope DTOs, and pure validation helpers.
- Keep compatibility re-exports in the existing crate while downstream modules migrate.
- Add checks that prevent the core crate from depending on adapter or CLI concerns.

## Impact

This change creates a compile-time modularity anchor. It should reduce accidental coupling and make later crate extractions smaller, while preserving current CLI and library behavior during migration.
