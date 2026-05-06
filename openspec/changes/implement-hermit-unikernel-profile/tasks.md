## Phase 1: Spec foundation

- [x] Create proposal, design, tasks, and delta spec for the Hermit unikernel profile implementation seam.

## Phase 2: Dependency and model alignment

- [ ] Align with `define-runtime-service-core` service identity, lifecycle, route, health, capability-binding, and receipt vocabulary before implementing host-specific effects.
- [ ] Add or update portable runtime model types for this runner/profile boundary.
- [ ] Add fail-closed admission checks for missing capabilities, invalid artifacts, denied handles, and unsupported profiles.

## Phase 3: Runner/profile implementation

- [ ] Implement the smallest Hermit profile/artifact/launch-compatibility surface while delegating generic VM lifecycle to the compatible microVM/Uhyve/loader runner.
- [ ] Emit secret-safe lifecycle, admission, output, and failure receipts.

## Phase 4: Tests and docs

- [ ] Add positive and negative tests for artifact verification, capability binding, lifecycle transitions, and receipt redaction.
- [ ] Update runtime architecture documentation or source-anchor tests if this change introduces new public terminology.
- [ ] Run focused tests, strict OpenSpec validation, and whitespace checks.
