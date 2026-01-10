# API Naming: get_ Prefix Violations (C-GETTER)

**Severity:** LOW
**Category:** Code Quality
**Date:** 2026-01-10

## Summary

147 methods use `get_` prefix on getter methods, violating Rust API Guidelines C-GETTER convention.

## Rust API Guidelines (C-GETTER)

Getter methods should NOT have the `get_` prefix:

- `fn get_repo()` should be `fn repo()`
- `fn get_metrics()` should be `fn metrics()`

## Affected Locations (Sample)

1. `crates/aspen-pijul/src/store.rs:27` - `get_repo()`
2. `crates/aspen-pijul/src/store.rs:31` - `get_channel()`
3. `crates/aspen-pijul/src/store.rs:35` - `get_change()`
4. `crates/aspen-testing/src/router.rs` - `get_raft_handle()`
5. `crates/aspen-cluster/src/federation/discovery.rs` - `get_discovered_clusters()`
6. `crates/aspen-cluster/src/federation/discovery.rs` - `get_seeders()`

## Reference

[Rust API Guidelines - C-GETTER](https://rust-lang.github.io/api-guidelines/naming.html#c-getter)

## Recommendation

Systematic refactoring to remove `get_` prefix:

- `get_repo()` -> `repo()`
- `get_channel()` -> `channel()`
- `get_change()` -> `change()`
- `get_raft_handle()` -> `raft_handle()`
- `get_metrics()` -> `metrics()`
- `get_discovered_clusters()` -> `discovered_clusters()`

This can be done incrementally without breaking API compatibility if internal methods.
