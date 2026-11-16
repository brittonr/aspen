## Phase 1: Typed product failure and observations

- [x] [serial] Preserve a typed pre-write-open EACCES from `NodeStateRoot` and annotate the ordinary node-status health write; generic and post-open failures remain non-qualifying. r[molten.testing.nixos_vm_fault_injection.storage_state_faults]
- [x] [serial] Add an optional outside-root node-status failure artifact from the actual failed operation, effective process identity/permissions and selected executable, leaving the command failed. Add positive and negative focused tests. r[molten.testing.nixos_vm_fault_injection.storage_state_faults]
- [x] [serial] Add read-only bounded VM state and permission observations, with separate pre/post captures of the protected root and target. r[molten.testing.nixos_vm_fault_injection.storage_state_faults]

## Phase 2: Validation and signed VM integration

- [x] [serial] Ingest actual canonical attempt/configuration/pre/post values in VM fault validation and check receipt linkage, target, selected package/build/binding/config, process identity, effective DAC permissions, EACCES/open-before-write and no protected mutation. Keep unavailable structural fixtures and add missing/handwritten/wrong-child/stale-state negatives. r[molten.testing.nixos_vm_fault_injection.fault_receipts]
- [x] [serial] Implement the signed VM scenario that invokes the existing status operation in node_b as the service identity with the root made unwritable, retains outside-root observations, and reports unavailable when required support is absent. Update executable fault docs. r[molten.testing.nixos_vm_fault_injection.storage_state_faults]

## Phase 3: Verification (Main-owned)

- [ ] [serial] Run focused typed-error, CLI and validator tests; format/lint changed Rust; evaluate the module and run opt-in signed VM rail, retain exact results and limitations. r[molten.testing.nixos_vm_fault_injection.storage_state_faults]
- [ ] [serial] Run strict Cairn validation/review and close only after observed evidence is retained; unavailable remains an explicit non-claim. r[molten.testing.nixos_vm_fault_injection.storage_state_faults] r[molten.testing.nixos_vm_fault_injection.fault_receipts]
