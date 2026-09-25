# Tasks

## Phase 1: Baseline and prerequisites

- [x] [serial] Record shared-runtime admission, WIT profile, and existing import-boundary baselines before core changes. r[aspen.wasm_import_admission.manifest]
- [x] [serial] Record accepted non-claim boundaries for component evidence and keep them unchanged. r[aspen.wasm_import_admission.nonclaims]

## Phase 2: Core and shell

- [x] [serial] Define manifest, world, import-set, verdict, and receipt DTOs in a focused pure family. r[aspen.wasm_import_admission.manifest] r[aspen.wasm_import_admission.evidence]
- [x] [serial] Implement pure manifest-shape validation, WIT world matching, import-set comparison, and receipt payload construction. r[aspen.wasm_import_admission.surface] r[aspen.wasm_import_admission.admission]
- [x] [serial] Add the host-side observation that records tool, verifier, and manifest identities without instantiating the component. r[aspen.wasm_import_admission.admission]

## Phase 3: Evidence and isolation

- [x] [parallel] Add positive exact-manifest, exact-world, and exact-import-set fixtures. r[aspen.wasm_import_admission.fixtures]
- [x] [parallel] Add negative drifting-world, undeclared-import, tool-drift, overclaim, and malformed-receipt fixtures. r[aspen.wasm_import_admission.fixtures] r[aspen.wasm_import_admission.evidence] r[aspen.wasm_import_admission.nonclaims]
- [x] [serial] Run focused tests before and after changes, Clippy with warnings denied, octet, Cairn validation and gates, and relevant Nix checks. r[aspen.wasm_import_admission.fixtures]
