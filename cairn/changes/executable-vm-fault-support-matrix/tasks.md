# Tasks: executable-vm-fault-support-matrix

## Phase 1: Support matrix model

- [ ] [parallel] r[molten.testing.nixos_vm.executable_fault_support_matrix] Add a pure executable VM fault support matrix model for fault kind, capability requirement, target, command profile, expected outcome, preflight refs, injection refs, child refs, post-fault refs, diagnostics, and caveats.
- [ ] [parallel] r[molten.testing.nixos_vm.executable_fault_validation_negatives] Add negative descriptor and receipt fixtures for unsupported pass, missing injection, missing child ref, missing denial diagnostic, wrong topology, tampered receipt, and log-only pass.

## Phase 2: VM support paths

- [ ] [serial] r[molten.testing.nixos_vm.executable_fault_support_matrix] Extend the VM check to emit support status and canonical receipts for available network, restart, filesystem, disk pressure, and receipt readback fault classes, with unavailable receipts when support is absent.
- [ ] [serial] r[molten.testing.nixos_vm.executable_fault_validation_negatives] Strengthen `fault-validate` coverage so every negative fixture denies before pass evidence.

## Phase 3: Review output

- [ ] [parallel] r[molten.testing.nixos_vm.executable_fault_support_matrix] Add reviewer documentation and support-table readback for realized VM fault evidence.
- [ ] [serial] r[molten.testing.nixos_vm.executable_fault_validation_negatives] Run focused VM fault validation tests and the smallest relevant VM check, or record host-support blockers and next best checks.
