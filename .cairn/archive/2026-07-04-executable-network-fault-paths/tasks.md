# Tasks: executable-network-fault-paths

## Phase 1: Capability and receipt core

- [x] [parallel] r[molten.testing.nixos_vm_fault_injection.network_control_probe] Define a pure network-control capability evidence model for backend, target link, topology ref, support status, cleanup strategy, diagnostics, and caveats.
- [x] [parallel] r[molten.testing.nixos_vm_fault_injection.network_fault_executable_path] Extend fault validation to require injection, child workflow, cleanup, topology, and post-fault refs for executable network pass evidence.

## Phase 2: VM shell execution

- [x] [serial] r[molten.testing.nixos_vm_fault_injection.network_control_probe] Add VM image or test-driver preflight commands that detect supported network-control backends and record unavailable evidence when none are present.
- [x] [serial] r[molten.testing.nixos_vm_fault_injection.network_fault_executable_path] Implement bounded delay, drop, partition, rejoin, and asymmetric latency fault runners for supported backends with deterministic cleanup evidence.

## Phase 3: Positive and negative coverage

- [x] [parallel] r[molten.testing.nixos_vm_fault_injection.network_fault_executable_path] Add positive executable-network fixtures for supported partition/rejoin and delay/drop runs that bind child workflow evidence.
- [x] [parallel] r[molten.testing.nixos_vm_fault_injection.network_fault_executable_path] Add negative fixtures for unsupported-host pass, missing injection, missing cleanup, unrejoined partition, stale topology, and log-only success.
- [x] [serial] r[molten.testing.nixos_vm_fault_injection.network_control_probe] Run focused fault validation tests plus the smallest executable VM network-fault check, or record host-support blockers and next best checks.
