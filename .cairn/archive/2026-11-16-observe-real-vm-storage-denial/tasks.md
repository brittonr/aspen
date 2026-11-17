## Phase 1: Typed product failure and observations

- [x] [serial] Preserve a typed pre-write-open EACCES from `NodeStateRoot` and annotate the ordinary node-status health write; generic and post-open failures remain non-qualifying. r[molten.testing.nixos_vm_fault_injection.storage_state_faults]
- [x] [serial] Add an optional outside-root node-status failure artifact from the actual failed operation, effective process identity/permissions and selected executable, leaving the command failed. Add positive and negative focused tests. r[molten.testing.nixos_vm_fault_injection.storage_state_faults]
- [x] [serial] Add read-only bounded VM state and permission observations, with separate pre/post captures of the protected root and target. r[molten.testing.nixos_vm_fault_injection.storage_state_faults]

## Phase 2: Validation and signed VM integration

- [x] [serial] Ingest actual canonical attempt/configuration/pre/post values in VM fault validation and check receipt linkage, target, selected package/build/binding/config, process identity, effective DAC permissions, EACCES/open-before-write and no protected mutation. Keep unavailable structural fixtures and add missing/handwritten/wrong-child/stale-state negatives. r[molten.testing.nixos_vm_fault_injection.fault_receipts]
- [x] [serial] Implement the signed VM scenario that invokes the existing status operation in node_b as the service identity with the root made unwritable, retains outside-root observations, and reports unavailable when required support is absent. Update executable fault docs. r[molten.testing.nixos_vm_fault_injection.storage_state_faults]

## Phase 3: Verification (Main-owned)

- [x] [serial] Run focused typed-error, CLI and validator tests; format/lint changed Rust; evaluate the module and run opt-in signed VM rail, retain exact results and limitations. r[molten.testing.nixos_vm_fault_injection.storage_state_faults]
- [x] [serial] Run strict Cairn validation/review and close only after observed evidence is retained; unavailable remains an explicit non-claim. r[molten.testing.nixos_vm_fault_injection.storage_state_faults] r[molten.testing.nixos_vm_fault_injection.fault_receipts]

## Main-owned verification

- [Retained Rust gates](../../../../evidence/integrated-cutover-2026-11-16/checks.json): both Octet runs, object-corpus/import/strict gate/remediation, workspace library tests, focused promotion CLI tests, Clippy with warnings denied, formatting and strict Cairn validation all exited zero. The completed VM below additionally executes the actual node-status CLI failure under the service UID.
- [Module VM observation](evidence/module-store-image-overlay-pass-2026-11-16.json): actual startup, restart, shutdown and six trust/source-denial controls passed for that separately recorded candidate.
- [Complete signed VM observation](evidence/signed-complete-pass-2026-11-16.json): the full `nixos-vm-multinode-signed` invocation exited zero; actual status health-file open failed with EACCES under UID/GID 998 and no effective capabilities, independently captured protected state matched, and canonical fault/VM/manifest validation passed with zero diagnostics.
- All 132 exported files in [the original packet](evidence/signed-vm-pass-20261116/) match the realized output byte-for-byte by BLAKE3. The publication guard scanned every file with no grandfathering or rejection. Original driver logs and a separate full packet copy remain operator-private.
- Strict Cairn validation, proposal/design gates and review completeness passed after preserving the prior general storage/state-fault and canonical-receipt obligations in the strengthened delta. Structural gate passes do not substitute for the actual VM result.

The claim is this exact signed VM-local fixture and selected binding. Independent producer/operator trust, production startup latency, arbitrary runtime/cross-host qualification, deterministic replay and release authority remain outside it. Earlier failed attempts remain retained unchanged.

