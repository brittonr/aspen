## Phase 1: Spec foundation

- [x] Create proposal, design, tasks, and delta spec for the Hermit unikernel profile implementation seam.

## Phase 2: Dependency and model alignment

- [x] Align with `define-runtime-service-core` service identity, lifecycle, route, health, capability-binding, and receipt vocabulary before implementing host-specific effects. ✅ 2m 21s (2026-05-07T02:18:25Z → 2026-05-07T02:20:46Z; evidence: `evidence/hermit-portable-model-admission.md`)
- [x] Add or update portable runtime model types for this runner/profile boundary. ✅ 2m 21s (2026-05-07T02:18:25Z → 2026-05-07T02:20:46Z; evidence: `evidence/hermit-portable-model-admission.md`)
- [x] Add fail-closed admission checks for missing capabilities, invalid artifacts, denied handles, and unsupported profiles. ✅ 2m 21s (2026-05-07T02:18:25Z → 2026-05-07T02:20:46Z; evidence: `evidence/hermit-portable-model-admission.md`)

## Phase 3: Runner/profile implementation

- [x] Implement the smallest Hermit profile/artifact/launch-compatibility surface while delegating generic VM lifecycle to the compatible microVM/Uhyve/loader runner. ✅ 2m 21s (2026-05-07T02:18:25Z → 2026-05-07T02:20:46Z; evidence: `evidence/hermit-portable-model-admission.md`)
- [x] Emit secret-safe lifecycle, admission, output, and failure receipts. ✅ 2m 21s (2026-05-07T02:18:25Z → 2026-05-07T02:20:46Z; evidence: `evidence/hermit-portable-model-admission.md`)

## Phase 4: Tests and docs

- [x] Add positive and negative tests for artifact verification, capability binding, lifecycle transitions, and receipt redaction. ✅ 2m 21s (2026-05-07T02:18:25Z → 2026-05-07T02:20:46Z; evidence: `evidence/hermit-portable-model-admission.md`)
- [ ] Update runtime architecture documentation or source-anchor tests if this change introduces new public terminology.
- [ ] Run focused tests, strict OpenSpec validation, and whitespace checks.
