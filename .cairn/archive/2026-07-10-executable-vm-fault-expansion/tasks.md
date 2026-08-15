# Tasks: executable-vm-fault-expansion

## Phase 1: Fault matrix core

- [x] [serial] r[molten.testing.executable_vm_fault_expansion.real_fault_matrix] Extend the VM fault support matrix and receipt validation for executable/probed fault cases.
- [x] [parallel] r[molten.testing.executable_vm_fault_expansion.unavailable_policy] Add diagnostics for unavailable network control, missing preflight, missing cleanup, simulated-only claims, log-only claims, and unsupported pass promotion.

## Phase 2: VM shell wiring

- [x] [serial] r[molten.testing.executable_vm_fault_expansion.real_fault_matrix] Wire executable or probed fault paths for network, restart, authority, duplicate/conflict, corrupted receipt, and state-root permission cases where host support permits.
- [x] [parallel] r[molten.testing.executable_vm_fault_expansion.unavailable_policy] Add unavailable-host fixtures and deny receipts for unsupported or unclean fault paths.

## Phase 3: Documentation and validation

- [x] [parallel] r[molten.testing.executable_vm_fault_expansion.real_fault_matrix] Document the executable, simulated, and unavailable fault evidence classes.
- [x] [serial] r[molten.testing.executable_vm_fault_expansion.unavailable_policy] Run focused VM fault tests, NixOS VM validation tests, and traceability coverage updates.
