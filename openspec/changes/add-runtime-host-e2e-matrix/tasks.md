## Phase 1: Matrix foundation

- [x] [serial] Write OpenSpec proposal, design, and delta spec for runtime-host E2E matrix coverage. ✅ 10m (started: 2026-05-07T17:20:00Z → completed: 2026-05-07T17:30:00Z)
- [x] [serial] Add `nested-kvm` suite prerequisite support to the Nickel schema and Rust inventory parser. ✅ 8m (started: 2026-05-07T17:30:00Z → completed: 2026-05-07T17:38:00Z)
- [x] [depends:nested-kvm] Register `vm-snapshot-e2e-test` as the first Aspen-spawned microVM runtime-host suite. ✅ 6m (started: 2026-05-07T17:38:00Z → completed: 2026-05-07T17:44:00Z)
- [x] [depends:runtime-host-microvm-suite] Regenerate and check committed test-harness inventory. ✅ 3m (started: 2026-05-07T17:44:00Z → completed: 2026-05-07T17:47:00Z)
- [x] [depends:runtime-host-microvm-suite] Capture focused validation evidence without launching the expensive nested-KVM VM run. ✅ 12m (started: 2026-05-07T17:47:00Z → completed: 2026-05-07T17:59:00Z)

## Phase 2: Follow-up host rows

- [ ] [depends:runtime-host-microvm-suite] Add a WASM runner E2E that executes through the real Aspen runtime path rather than only plugin install/reload plumbing.
- [ ] [depends:runtime-host-microvm-suite] Add an OCI lowering E2E that ingests an immutable OCI artifact, lowers it into an isolated host, executes it, and reads a secret-safe receipt.
- [ ] [depends:runtime-host-microvm-suite] Add Hyperlight/Hermit E2E rows once their runner paths are stable enough for bounded nested-KVM verification.
