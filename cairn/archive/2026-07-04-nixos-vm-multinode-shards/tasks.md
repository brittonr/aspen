# Tasks: nixos-vm-multinode-shards

## Phase 1: Shard plan core

- [x] [parallel] r[molten.testing.nixos_vm_multinode.sharded_checks] Define a pure VM shard plan model for scenario fixture refs, node set, required receipt kinds, expected artifacts, unavailable policy, diagnostics, and caveats.
- [x] [parallel] r[molten.testing.nixos_vm_multinode.shard_aggregate] Define a pure aggregate model that accepts child shard refs and rejects missing, denied, unavailable-as-pass, stale, or log-only shard evidence.

## Phase 2: Nix integration

- [x] [serial] r[molten.testing.nixos_vm_multinode.sharded_checks] Split the existing monolithic VM script into smoke, live-control, service-job, restart, and VM-fault shard checks while preserving the current full coverage path.
- [x] [serial] r[molten.testing.nixos_vm_multinode.shard_aggregate] Add an aggregate VM check output that preserves child shard receipts, manifest entries, validation receipts, and diagnostic logs.

## Phase 3: Positive and negative coverage

- [x] [parallel] r[molten.testing.nixos_vm_multinode.sharded_checks] Add positive shard tests or fixtures proving each shard emits the declared canonical receipt set.
- [x] [parallel] r[molten.testing.nixos_vm_multinode.shard_aggregate] Add negative fixtures for missing shard, denied shard, unavailable-as-pass, stale child ref, and log-only child evidence.
- [x] [serial] r[molten.testing.nixos_vm_multinode.shard_aggregate] Run focused shard validator tests plus the smallest available VM shard, or record host-support blockers and next best checks.
