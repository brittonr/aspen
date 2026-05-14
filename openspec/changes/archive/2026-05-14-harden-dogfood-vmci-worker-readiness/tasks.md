## Phase 1: Blocker capture

- [x] [serial] Capture current-head VM-CI dogfood attempt evidence and root blocker.
- [x] [serial] Create scoped OpenSpec for fail-fast VM-CI worker readiness.

## Phase 2: Readiness model

- [x] [serial] Identify the VM pool readiness state and the dogfood/CI wait seam that can observe zero eligible VM workers.
- [x] [depends:readiness-seam] Define a bounded failure category for TAP/TUN/KVM/image/workspace provisioning failures.

## Phase 3: Implementation

- [x] [depends:readiness-model] Add fail-fast VM-CI readiness handling before or immediately after pipeline trigger.
- [x] [depends:fail-fast] Persist a failed receipt stage with redacted VM worker readiness diagnostics.
- [x] [depends:receipt] Add tests covering zero-capacity VM worker failure and non-regression for successful VM-CI readiness.

## Phase 4: Verification

- [x] [depends:tests] Run focused tests and strict OpenSpec validation; host-capability VM-CI proof remains environment-gated when TAP/TUN permissions are available.
