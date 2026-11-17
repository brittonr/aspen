## Why

Issue #24 found that the signed VM rail's storage fault could claim an observed permission denial without executing a protected operation. The interim unavailable classification is truthful but leaves a locally implementable execution-evidence gap. `node status` already makes a real capability-rooted health-file write; the node-state filesystem currently erases the OS error kind before the CLI could retain canonical failure evidence.

## What Changes

- Preserve a typed `PermissionDenied` failure only when the real `NodeStateRoot` health-file open fails before any write or truncate; the normal node-status command still fails. Publish a canonical, versioned operation attempt outside the protected root, deriving its failure from that syscall and its effective identity/permissions from the executing process. r[molten.testing.nixos_vm_fault_injection.storage_state_faults]
- On node_b, after positive signed startup and shutdown, make the state root unwritable to the non-root service identity, independently inspect the protected tree before and after the actual `node status` attempt, and retain the selected package, binding, configuration, executing identity, target and permission observations. Never treat an unavailable namespace/identity setup as an observed denial. r[molten.testing.nixos_vm_fault_injection.storage_state_faults]
- Reuse `nixos-vm-fault-descriptor-v1` and `nixos-vm-fault-receipt-v1` refs while making `fault-validate` ingest actual canonical child and observation values. Accept the observed-denial status only for a matching typed pre-write permission failure, denied effective permissions, absent target, and identical independently measured protected state. Reject missing, handwritten, stale or unrelated refs. Keep structural unavailable fixtures. r[molten.testing.nixos_vm_fault_injection.fault_receipts]

## Impact

- **Files:** node-host error and filesystem authority, daemon status, CLI node-status and NixOS VM fault inputs, VM validation/fixtures, signed VM test script and executable-fault documentation.
- **Testing:** focused typed-error/receipt tests, positive and negative VM fault validator fixtures, module and signed VM rails on a capable host. Verification remains open; source changes alone are not execution evidence.
- **Non-goals:** no fake storage command, root DAC bypass, independent producer/operator qualification, general filesystem error receipt, WAN or deployment claim, or source-trust relaxation.
