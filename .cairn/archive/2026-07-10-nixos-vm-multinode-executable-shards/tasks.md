# Tasks: nixos-vm-multinode-executable-shards

## Phase 1: Nix shard surfaces

- [x] [serial] r[molten.testing.nixos_vm_multinode.executable_shard_derivations] Split the monolithic VM script into executable smoke, live-control, service/job, restart, and fault check attributes or apps.
- [x] [serial] r[molten.testing.nixos_vm_multinode.executable_shard_derivations] Ensure each shard writes a `nixos-vm-shard-run-v1` receipt plus its required child receipts and diagnostic logs.

## Phase 2: Aggregate check

- [x] [serial] r[molten.testing.nixos_vm_multinode.executable_shard_aggregate] Implement an aggregate check that consumes realized child shard outputs and emits `nixos-vm-multinode-aggregate-v1`.
- [x] [parallel] r[molten.testing.nixos_vm_multinode.executable_shard_aggregate] Add negative aggregate fixtures for missing shard, denied shard, stale child, unavailable-as-pass, and log-only child evidence.

## Phase 3: Documentation and validation

- [x] [parallel] r[molten.testing.nixos_vm_multinode.executable_shard_derivations] Document shard command surfaces and expected output artifacts.
- [x] [serial] r[molten.testing.nixos_vm_multinode.executable_shard_aggregate] Run pure shard/aggregate tests plus the smallest executable VM shard available on the host, recording host-support blockers when unavailable.
