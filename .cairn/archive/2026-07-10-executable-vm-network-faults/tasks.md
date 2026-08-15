# Tasks: executable-vm-network-faults

## Phase 1: Backend support model

- [x] [parallel] r[molten.testing.nixos_vm_fault_injection.executable_network_backend] Add or extend network-control probe receipts to declare backend id, target link, cleanup strategy, support status, diagnostics, and caveats.
- [x] [parallel] r[molten.testing.nixos_vm_fault_injection.network_fault_cleanup_validation] Extend pure fault validation to require cleanup and post-fault refs for network-fault pass evidence.

## Phase 2: Executable VM path

- [x] [serial] r[molten.testing.nixos_vm_fault_injection.executable_network_backend] Implement bounded executable delay, drop, partition, rejoin, and asymmetric-latency fault paths for supported VM backends.
- [x] [serial] r[molten.testing.nixos_vm_fault_injection.network_fault_cleanup_validation] Emit cleanup and post-fault evidence before any network-fault pass receipt is accepted.

## Phase 3: Fixtures and validation

- [x] [parallel] r[molten.testing.nixos_vm_fault_injection.executable_network_backend] Add unavailable-backend fixtures that remain non-pass evidence.
- [x] [parallel] r[molten.testing.nixos_vm_fault_injection.network_fault_cleanup_validation] Add negative fixtures for missing cleanup, missing child workflow, wrong topology, unavailable-as-pass, and log-only claims.
- [x] [serial] r[molten.testing.nixos_vm_fault_injection.executable_network_backend] Run pure fault validation tests plus executable VM network-fault checks when host support is available.
