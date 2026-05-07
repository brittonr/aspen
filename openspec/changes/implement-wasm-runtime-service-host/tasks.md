## Phase 1: Spec foundation

- [x] Create proposal, design, tasks, and delta spec for the WASM runtime service host implementation seam.

## Phase 2: Dependency and model alignment

- [x] Align with `define-runtime-service-core` service identity, lifecycle, route, health, capability-binding, and receipt vocabulary before implementing host-specific effects. ✅ evidence: `openspec/changes/implement-wasm-runtime-service-host/evidence/wasm-portable-model-admission.md`
- [x] Add or update portable runtime model types for this runner/profile boundary. ✅ evidence: `openspec/changes/implement-wasm-runtime-service-host/evidence/wasm-portable-model-admission.md`
- [x] Add fail-closed admission checks for missing capabilities, invalid artifacts, denied handles, and unsupported profiles. ✅ evidence: `openspec/changes/implement-wasm-runtime-service-host/evidence/wasm-portable-model-admission.md`

## Phase 3: Runner/profile implementation

- [x] Implement the smallest WASM host ABI and instantiation surface needed to validate, instantiate, call, stop, and observe a module without broad scheduler work. ✅ evidence: `openspec/changes/implement-wasm-runtime-service-host/evidence/wasm-portable-model-admission.md`
- [x] Emit secret-safe lifecycle, admission, output, and failure receipts. ✅ evidence: `openspec/changes/implement-wasm-runtime-service-host/evidence/wasm-portable-model-admission.md`

## Phase 4: Tests and docs

- [x] Add positive and negative tests for artifact verification, capability binding, lifecycle transitions, and receipt redaction. ✅ evidence: `openspec/changes/implement-wasm-runtime-service-host/evidence/wasm-portable-model-admission.md`
- [ ] Update runtime architecture documentation or source-anchor tests if this change introduces new public terminology.
- [ ] Run focused tests, strict OpenSpec validation, and whitespace checks.
