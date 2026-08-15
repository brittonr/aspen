# Tasks: nixos-vm-executable-fault-injection

## Phase 1: Fault descriptors and support checks

- [x] [serial] r[molten.testing.nixos_vm_fault_injection.fault_descriptors] Define canonical VM fault descriptors and host-support/unavailable evidence for executable network, process, storage, and restart faults.
- [x] [serial] r[molten.testing.nixos_vm_fault_injection.unavailable_boundary] Ensure missing KVM, test-driver, network, or privilege support records unavailable/deny evidence and never counts as pass evidence.

## Phase 2: Executable VM faults

- [x] [parallel] r[molten.testing.nixos_vm_fault_injection.network_faults] Add executable delay, drop, partition, and rejoin cases to the NixOS VM topology where supported.
- [x] [parallel] r[molten.testing.nixos_vm_fault_injection.restart_windows] Add crash/restart windows around queued control requests, partial dispatch, duplicate send, and receipt write/readback paths.
- [x] [parallel] r[molten.testing.nixos_vm_fault_injection.storage_state_faults] Add bounded missing-artifact, permission-denied, and state-root pressure cases where deterministic VM support permits.

## Phase 3: Receipts, validation, and docs

- [x] [serial] r[molten.testing.nixos_vm_fault_injection.fault_receipts] Emit and validate canonical VM fault receipts with preflight, injection, observation, child refs, decisions, diagnostics, and caveats.
- [x] [parallel] r[molten.testing.nixos_vm_fault_injection.negative_fixtures] Add negative fixtures for unsupported host features, stale evidence, tampered fault receipts, wrong topology, and log-only pass claims.
- [x] [serial] r[molten.testing.nixos_vm_fault_injection.docs] Document executable VM fault checks, host requirements, unavailable handling, and evidence boundaries.
