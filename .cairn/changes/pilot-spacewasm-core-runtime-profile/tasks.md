## Tasks

- [ ] [serial] Define the typed Nickel `spacewasm-core-mvp-pilot` profile, exact runtime/materialization/evidence cohort, module/ABI/resource limits, segment/yield policy, retention, promotion state, and non-claims. r[molten.spacewasm_core.profile] r[molten.spacewasm_core.materialization] r[molten.spacewasm_core.resources]
- [ ] [depends:spacewasm-core-profile] Implement pure profile, bundle, Octet/Chaos evidence, module feature, import/export, ABI, authority, resource, and promotion admission with deterministic blockers. r[molten.spacewasm_core.functional_core] r[molten.spacewasm_core.admission]
- [ ] [depends:spacewasm-core-profile] Define continuation, segment-plan, execution-observation, replay-comparison, and receipt DTOs with generation/artifact/profile/input/effect bindings. r[molten.spacewasm_core.resume] r[molten.spacewasm_core.receipts]
- [ ] [depends:spacewasm-core-admission] Add a thin SpaceWasm shell that consumes a verified Mantle bundle, streams exact module bytes, links only declared `molten:hostcall` functions, and implements bounded `molten.wasm.abi.v1` Preserves memory transfer. r[molten.spacewasm_core.execution] r[molten.spacewasm_core.abi]
- [ ] [depends:spacewasm-core-shell] Enforce separate interpreter/code/stack and guest-linear-memory/table/growth/instruction bounds with typed OOM, denial, trap, and harness outcomes. r[molten.spacewasm_core.resources]
- [ ] [depends:spacewasm-core-shell] Add generation-fenced finish/trap/out-of-fuel/host-pause handling, bounded continuation retention, stale-resume denial, and explicit cleanup. r[molten.spacewasm_core.resume]
- [ ] [depends:spacewasm-core-resume] Emit inspection, admission, instantiation, segment, yield/resume, hostcall, resource, terminal, replay, and denial receipts with engine-specific identities and non-claims. r[molten.spacewasm_core.receipts]
- [ ] [parallel] Add positive exact-bundle, admitted MVP, Preserves ABI round-trip, declared hostcall, expected trap, out-of-fuel/resume, uninterrupted/segmented replay, and cleanup fixtures plus negative missing/stale evidence, unsupported proposal, component/WASI, undeclared import, wrong ABI, invalid pointer/length, malformed output, memory/growth/stack/instruction exhaustion, stale continuation, host trap, replay mismatch, fallback, and overclaim fixtures. r[molten.spacewasm_core.fixtures]
- [ ] [depends:spacewasm-core-receipts] Add operator status/readback that keeps SpaceWasm pilot, Wasmtime core, and component evidence distinct and reports promotion blockers. r[molten.spacewasm_core.nonclaims]
- [ ] [depends:spacewasm-core-fixtures] Run focused pure-core/shell/ABI/resource/resume/replay tests, executor conformance, positive and negative fixtures, Octet and Chaos evidence checks, Cairn validation, proposal/design/tasks gates, and the smallest relevant workspace/Nix checks. r[molten.spacewasm_core.validation]

## Prerequisite status (2026-11-16)

The engine prerequisite is closed: Molten and the shared hardened runtime are both on the
patched Wasmtime 49.0.5 cohort (archived `adopt-patched-wasmtime-line`), so the SpaceWasm
differential no longer needs an advisory exception. Remaining before this change can execute
evidence-bearing acceptance:

- Mantle's independently admitted immutable revision/bundle publication binding. Its 2026-10-30 local integration verified bundle `774331f6c2defe75e30c354b2b3080838cf56438bbd99268779fe4590f8c9f39`, but `materialize-molten-spacewasm-abi` still leaves publication unchecked; a local materialization is not an authorized trust pin;
- exact-member static receipts and independently selected immutable verifier identity for Octet's archived `spacewasm-molten-abi-pilot` profile (`b3:2fd08aba3959f85f58aaac323758df3e286872cf14e7b52985a1b8d7c4647956`);
- a compatible ChaosControl full-Molten-ABI differential producer/report and independent verifier/probe pins. The inspected archived/current rail is hostless, uses `chaoscontrol.spacewasm-mvp-differential-report.v1` and Wasmtime 41.0.3; it is not the required `chaoscontrol.spacewasm-molten-abi-differential.v1` / Wasmtime 49.0.5 cohort.
