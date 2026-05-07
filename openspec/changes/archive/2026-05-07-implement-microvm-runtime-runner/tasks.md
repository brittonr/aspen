## Phase 1: Spec foundation

- [x] Create proposal, design, tasks, and delta spec for the microVM runtime runner implementation seam.

## Phase 2: Dependency and model alignment

- [x] Align with `define-runtime-service-core` service identity, lifecycle, route, health, capability-binding, and receipt vocabulary before implementing host-specific effects. ✅ 2m 37s (2026-05-07T02:53:07Z → 2026-05-07T02:55:44Z; evidence: `evidence/microvm-portable-model-admission.md`)
- [x] Add or update portable runtime model types for this runner/profile boundary. ✅ 2m 37s (2026-05-07T02:53:07Z → 2026-05-07T02:55:44Z; evidence: `evidence/microvm-portable-model-admission.md`)
- [x] Add fail-closed admission checks for missing capabilities, invalid artifacts, denied handles, and unsupported profiles. ✅ 2m 37s (2026-05-07T02:53:07Z → 2026-05-07T02:55:44Z; evidence: `evidence/microvm-portable-model-admission.md`)

## Phase 3: Runner/profile implementation

- [x] Implement the smallest node-local runner/profile surface needed to prepare, start, stop, and observe the unit without broad scheduler work. ✅ 2m 37s (2026-05-07T02:53:07Z → 2026-05-07T02:55:44Z; evidence: `evidence/microvm-portable-model-admission.md`)
- [x] Emit secret-safe lifecycle, admission, output, and failure receipts. ✅ 2m 37s (2026-05-07T02:53:07Z → 2026-05-07T02:55:44Z; evidence: `evidence/microvm-portable-model-admission.md`)

## Phase 4: Tests and docs

- [x] Add positive and negative tests for artifact verification, capability binding, lifecycle transitions, and receipt redaction. ✅ 2m 37s (2026-05-07T02:53:07Z → 2026-05-07T02:55:44Z; evidence: `evidence/microvm-portable-model-admission.md`)
- [x] Update runtime architecture documentation or source-anchor tests if this change introduces new public terminology. ✅ 49s (2026-05-07T02:56:29Z → 2026-05-07T02:57:18Z; evidence: `evidence/microvm-runtime-docs.md`)
- [x] Run focused tests, strict OpenSpec validation, and whitespace checks. ✅ 25s (2026-05-07T02:57:44Z → 2026-05-07T02:58:09Z; evidence: `evidence/final-validation.md`)
