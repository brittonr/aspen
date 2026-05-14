## Phase 1: Blocker capture

- [x] [serial] Capture current-head VM-CI dogfood attempt evidence and root blocker.
- [x] [serial] Create scoped OpenSpec for fail-fast VM-CI worker readiness.

## Phase 2: Readiness model

- [ ] [serial] Identify the VM pool readiness state and the dogfood/CI wait seam that can observe zero eligible VM workers.
- [ ] [depends:readiness-seam] Define a bounded failure category for TAP/TUN/KVM/image/workspace provisioning failures.

## Phase 3: Implementation

- [ ] [depends:readiness-model] Add fail-fast VM-CI readiness handling before or immediately after pipeline trigger.
- [ ] [depends:fail-fast] Persist a failed receipt stage with redacted VM worker readiness diagnostics.
- [ ] [depends:receipt] Add tests covering zero-capacity VM worker failure and non-regression for successful VM-CI readiness.

## Phase 4: Verification

- [ ] [depends:tests] Run focused tests, strict OpenSpec validation, and a host-capability VM-CI proof where TAP/TUN permissions are available.
