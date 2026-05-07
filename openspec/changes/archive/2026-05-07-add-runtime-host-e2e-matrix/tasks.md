## Phase 1: Matrix foundation

- [x] [serial] Write OpenSpec proposal, design, and delta spec for runtime-host E2E matrix coverage. ✅ 10m (started: 2026-05-07T17:20:00Z → completed: 2026-05-07T17:30:00Z)
- [x] [serial] Add `nested-kvm` suite prerequisite support to the Nickel schema and Rust inventory parser. ✅ 8m (started: 2026-05-07T17:30:00Z → completed: 2026-05-07T17:38:00Z)
- [x] [depends:nested-kvm] Register `vm-snapshot-e2e-test` as the first Aspen-spawned microVM runtime-host suite. ✅ 6m (started: 2026-05-07T17:38:00Z → completed: 2026-05-07T17:44:00Z)
- [x] [depends:runtime-host-microvm-suite] Regenerate and check committed test-harness inventory. ✅ 3m (started: 2026-05-07T17:44:00Z → completed: 2026-05-07T17:47:00Z)
- [x] [depends:runtime-host-microvm-suite] Capture focused validation evidence without launching the expensive nested-KVM VM run. ✅ 12m (started: 2026-05-07T17:47:00Z → completed: 2026-05-07T17:59:00Z)

## Phase 2: Follow-up host rows

- [x] [depends:runtime-host-microvm-suite] Add explicit runtime-host matrix metadata support for host kind, proof level, support status, and metadata-only gap rows. ✅ 24m (started: 2026-05-07T18:20:00Z → completed: 2026-05-07T18:44:00Z)
- [x] [depends:runtime-host-metadata] Add WASM, OCI lowering, Hyperlight, and Hermit/unikernel metadata-only gap rows without overclaiming E2E execution. ✅ 10m (started: 2026-05-07T18:34:00Z → completed: 2026-05-07T18:44:00Z)
- [x] [depends:runtime-host-gap-rows] Regenerate inventory and validate OpenSpec, harness, Rust, command-list, and gated Nix metadata checks before archive. ✅ 12m (started: 2026-05-07T18:44:00Z → completed: 2026-05-07T18:56:00Z)
