# Unsafe Type Transmutation Pattern

**Severity:** MEDIUM
**Category:** Code Safety
**Date:** 2026-01-10

## Summary

The codebase uses `std::mem::transmute` in 4 locations to convert between structurally identical Raft type configurations. While documented, this pattern is fragile.

## Locations

1. **src/bin/aspen-node.rs:1186, 1199**

```rust
let transport_raft: openraft::Raft<aspen_transport::rpc::AppTypeConfig> =
    unsafe { std::mem::transmute(handle.storage.raft_node.raft().as_ref().clone()) };
```

2. **src/node/mod.rs:424, 581**

```rust
let raft_core_transport: openraft::Raft<aspen_transport::rpc::AppTypeConfig> =
    unsafe { std::mem::transmute(raft_core.clone()) };
```

3. **crates/aspen-cluster/src/bootstrap.rs:1077**

```rust
let transport_raft: openraft::Raft<TransportAppTypeConfig> =
    unsafe { std::mem::transmute(raft.as_ref().clone()) };
```

## Risk

- If `aspen_raft::types::AppTypeConfig` and `aspen_transport::rpc::AppTypeConfig` ever diverge, undefined behavior occurs silently
- Field order changes, type sizes, or alignment differences cause memory corruption
- No compile-time verification of type compatibility

## Recommendation

1. Define a single `AppTypeConfig` type in a shared crate
2. Re-export from both locations using type aliases
3. Or implement proper `From`/`Into` conversions
4. Add compile-time layout assertions using `static_assertions` crate:

```rust
static_assertions::assert_eq_size!(
    openraft::Raft<aspen_raft::types::AppTypeConfig>,
    openraft::Raft<aspen_transport::rpc::AppTypeConfig>
);
```
