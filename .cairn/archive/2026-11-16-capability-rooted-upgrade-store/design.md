## Context

`src/upgrades` used a private `mod fs` that forwarded to `std::fs`. Store locations were `root.join(...)` paths. `write_preserves` called `fs::write`, which truncates in place; status files were written the same way. Recursive scans used `read_dir` and `entry.path()`, so a link inside the store was followed. Other stores already use typed `local_store` roots over a held `cap_std` directory.

## Decisions

### Decision: A typed store root, opened once at the boundary

**Choice:** add `Kind::Upgrade` and `UpgradeStoreRoot` through the existing `typed_root!` macro. `molten upgrade` opens it from `--store`; library functions take `&UpgradeStoreRoot`. Ledger and registry parameters stay separate authorities.

**Rationale:** this follows the established typed-root pattern, makes the store authority explicit in every signature, and confines all descendant I/O to the held capability.

### Decision: Staged publication with immutability for content-addressed members

**Choice:** plans and receipts use `publish_new`; when the member already exists the bytes must match, otherwise the call reports a conflict. Pointers and status files use `publish` (stage, synchronize, rename, synchronize the directory). The pointer receipt is stored first.

**Rationale:** a crash leaves every member absent, previous or whole, and an existing content-addressed member can never be silently rewritten. Storing the receipt first means a visible pointer always has its receipt.

### Decision: Keep the layout; hide stages from listings

**Choice:** relative paths and `blake3_<hex>.preserves` names are unchanged, `StorageRoot::list_entries` skips publication stages, and upgrades scans refuse links and non-regular members.

**Rationale:** existing stores stay readable and the state snapshot reference keeps its meaning. A stage left by a crash cannot change a snapshot reference or be parsed as a pointer.

## Risks / Trade-offs

- The operator-selected root itself is opened through ambient authority once, as for other typed stores.
- A multi-file upgrade task is still not transactional; a crash between a task's effect and its receipt leaves the effect without a completion status, which the existing receipt-backed status check reports as incomplete.
- Publication observes payload and directory synchronization, not hardware durability.
