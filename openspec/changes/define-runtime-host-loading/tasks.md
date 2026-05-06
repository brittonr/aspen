## Phase 1: Spec foundation

- [x] Create proposal, design, delta spec, and task rail for `define-runtime-host-loading`.

## Phase 2: Runtime core type design

- [x] Define portable runtime-core model types for `RuntimeHostKind`, `RuntimeArtifact`, `RuntimeUnitKind`, `RuntimeCapabilityBinding`, `RuntimeResources`, route ownership, lifecycle status, and receipts.
- [x] Add serialization, redaction, and round-trip tests for runtime host-loading types.
- [x] Document the host taxonomy in `docs/runtime-applications.md` and keep a docs/source-anchor test for Native/WASM/Hyperlight loading.

## Phase 3: Native built-in service loading

- [x] Add a native built-in service factory registry shape for first-party services without introducing in-process native dynamic plugins.
- [x] Wrap Forge as `BuiltIn("forge")` in the runtime model while preserving current Forge internals.
- [x] Move or mirror Forge route registration through a runtime-owned route declaration path and emit bounded startup/route receipts.

## Phase 4: Dynamic host planning

- [x] Define WASM artifact loading plans that verify module hash/signature, ABI, entrypoint, fuel, memory, timeout, and capability bindings before instantiation.
- [x] Define Hyperlight execution-run loading plans that verify artifacts, assign to compatible runners, attach leases/heartbeats/logs, and record output artifacts/receipts.
- [x] Define external native process loading as a future trusted-operator path with verified binary identity and IPC/host-ABI boundary.

## Phase 5: Capability, UCAN, and verified-logic integration

- [x] Inspect `../ucan/` and record which ability/resource/proof/caveat concepts map cleanly to Aspen runtime capability bindings.
- [x] Inspect `../verified-logic/` and record candidate finite admission predicates for host-kind, artifact-hash shape, resource-bound shape, ability/resource syntax, proof-hop depth, and caveat payload shape.
- [x] Implement the first narrow verified-admission bridge only after choosing a finite predicate whose boundary is structural rather than cryptographic/runtime/I/O dependent.
- [x] Add negative tests proving denied/invalid capabilities fail closed and raw secrets never appear in manifests, logs, or receipts.

## Phase 6: Verification and closeout

- [x] Run focused Rust tests for runtime-core/native-loading changes.
- [x] Run relevant docs tests and `git diff --check`.
- [x] Run strict OpenSpec validation and helper verification.
- [x] Sync/archive only after implementation, evidence, docs, UCAN/verified-logic review notes, and verification tasks are complete.
