## Why

Kiln and other consumers need qualified compute offers independently of tool workers and running attempts. Existing fabric membership capacity covers CPU, memory, and storage but does not account for overlapping accelerator views or uncertain backend release.

## What Changes

- Define separate versioned compute offers, physical backing pools, qualified compatibility, and deterministic allocation/release transitions without changing membership records.
- Compose the pure core with a capability-rooted Redb owner, admitted effect ports, canonical Preserves records, supplied-observation profiles and synthetic consumer guidance. Live device/provider qualification remains a separate task.

## Impact

- **Files**: `crates/molten-core/src/compute_capacity/`, `src/compute_capacity/`, `src/lib.rs`, `config/compute-capacity/`, `docs/compute-capacity.md`, architecture and index links, `.cairn/changes/add-compute-capacity-service/`.
- **Testing**: focused core and root-crate tests, Redb reopen/interruption scenarios, strict canonical parser and Nickel checks, root-crate compilation, nightly formatting, strict Cairn validation and proposal/design/tasks gates.
