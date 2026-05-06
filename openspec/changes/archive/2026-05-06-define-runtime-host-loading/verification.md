# Verification: define-runtime-host-loading

## Implementation Evidence

- Changed file: `Cargo.toml`
- Changed file: `crates/aspen-runtime-core/Cargo.toml`
- Changed file: `crates/aspen-runtime-core/src/lib.rs`
- Changed file: `docs/runtime-applications.md`
- Changed file: `tests/runtime_host_loading_docs_test.rs`
- Changed file: `openspec/changes/define-runtime-host-loading/proposal.md`
- Changed file: `openspec/changes/define-runtime-host-loading/design.md`
- Changed file: `openspec/changes/define-runtime-host-loading/tasks.md`
- Changed file: `openspec/changes/define-runtime-host-loading/verification.md`
- Changed file: `openspec/changes/define-runtime-host-loading/evidence/runtime-core-tests.txt`
- Changed file: `openspec/changes/define-runtime-host-loading/evidence/docs-source-anchor-tests.txt`
- Changed file: `openspec/changes/define-runtime-host-loading/evidence/runtime-core-dependency-tree.txt`
- Changed file: `openspec/changes/define-runtime-host-loading/evidence/ucan-verified-logic-review.md`
- Changed file: `openspec/changes/define-runtime-host-loading/evidence/hermit-runtime-reference-review.md`

The drained slice adds a portable `aspen-runtime-core` crate with data-only runtime host-loading contracts and pure admission checks. It deliberately does not start processes, load WASM/Hyperlight/OCI/microVM artifacts, perform cryptography, or touch network/filesystem policy backends.

## Task Coverage

- [x] Create proposal, design, delta spec, and task rail for `define-runtime-host-loading`.
  - Evidence: `openspec/changes/define-runtime-host-loading/proposal.md`.
  - Evidence: `openspec/changes/define-runtime-host-loading/design.md`.
  - Evidence: `openspec/changes/define-runtime-host-loading/specs/runtime-host-loading/spec.md`.
  - Evidence: `openspec/changes/define-runtime-host-loading/tasks.md`.
- [x] Define portable runtime-core model types for `RuntimeHostKind`, `RuntimeArtifact`, `RuntimeUnitKind`, `RuntimeCapabilityBinding`, `RuntimeResources`, route ownership, lifecycle status, and receipts.
  - Evidence: `crates/aspen-runtime-core/src/lib.rs`.
- [x] Add serialization, redaction, and round-trip tests for runtime host-loading types.
  - Evidence: `openspec/changes/define-runtime-host-loading/evidence/runtime-core-tests.txt`.
- [x] Document the host taxonomy in `docs/runtime-applications.md` and keep a docs/source-anchor test for Native/WASM/Hyperlight loading.
  - Evidence: `docs/runtime-applications.md`; `tests/runtime_host_loading_docs_test.rs`; `openspec/changes/define-runtime-host-loading/evidence/docs-source-anchor-tests.txt`.
- [x] Add a native built-in service factory registry shape for first-party services without introducing in-process native dynamic plugins.
  - Evidence: `NativeBuiltInServiceFactory` in `crates/aspen-runtime-core/src/lib.rs`.
- [x] Wrap Forge as `BuiltIn("forge")` in the runtime model while preserving current Forge internals.
  - Evidence: `native_factory_wraps_forge_as_builtin` test in `crates/aspen-runtime-core/src/lib.rs`.
- [x] Move or mirror Forge route registration through a runtime-owned route declaration path and emit bounded startup/route receipts.
  - Evidence: `RuntimeRouteDeclaration`, `RuntimeReceipt`, and Forge test route declaration.
- [x] Define WASM artifact loading plans that verify module hash/signature, ABI, entrypoint, fuel, memory, timeout, and capability bindings before instantiation.
  - Evidence: `RuntimeArtifact::WasmModule`, `RuntimeResources`, docs taxonomy, and spec/design plan.
- [x] Define Hyperlight execution-run loading plans that verify artifacts, assign to compatible runners, attach leases/heartbeats/logs, and record output artifacts/receipts.
  - Evidence: `RuntimeHostKind::Hyperlight`, `RuntimeArtifact::HyperlightImage`, `RuntimeReceipt`, docs taxonomy, and spec/design plan.
- [x] Define external native process loading as a future trusted-operator path with verified binary identity and IPC/host-ABI boundary.
  - Evidence: `RuntimeHostKind::NativeProcess`, `RuntimeArtifact::NativeBinary`, docs taxonomy, and spec/design plan.
- [x] Inspect `../ucan/` and record which ability/resource/proof/caveat concepts map cleanly to Aspen runtime capability bindings.
  - Evidence: `openspec/changes/define-runtime-host-loading/evidence/ucan-verified-logic-review.md`.
- [x] Inspect `../verified-logic/` and record candidate finite admission predicates for host-kind, artifact-hash shape, resource-bound shape, ability/resource syntax, proof-hop depth, and caveat payload shape.
  - Evidence: `openspec/changes/define-runtime-host-loading/evidence/ucan-verified-logic-review.md`.
- [x] Implement the first narrow verified-admission bridge only after choosing a finite predicate whose boundary is structural rather than cryptographic/runtime/I/O dependent.
  - Evidence: `admit_unit`, `admit_receipt`, and negative tests in `crates/aspen-runtime-core/src/lib.rs`.
- [x] Add negative tests proving denied/invalid capabilities fail closed and raw secrets never appear in manifests, logs, or receipts.
  - Evidence: `admission_rejects_unsafe_shapes`, `receipts_redact_raw_secrets` tests.
- [x] Run focused Rust tests for runtime-core/native-loading changes.
  - Evidence: `openspec/changes/define-runtime-host-loading/evidence/runtime-core-tests.txt`.
- [x] Run relevant docs tests and `git diff --check`.
  - Evidence: `openspec/changes/define-runtime-host-loading/evidence/docs-source-anchor-tests.txt`; this file.
- [x] Run strict OpenSpec validation and helper verification.
  - Evidence: this file.
- [x] Sync/archive only after implementation, evidence, docs, UCAN/verified-logic review notes, and verification tasks are complete.
  - Evidence: this file and archive commit.

## Verification Commands

### `cargo test -p aspen-runtime-core`
- Status: pass
- Artifact: `openspec/changes/define-runtime-host-loading/evidence/runtime-core-tests.txt`

### `cargo test --test runtime_host_loading_docs_test`
- Status: pass
- Artifact: `openspec/changes/define-runtime-host-loading/evidence/docs-source-anchor-tests.txt`

### `cargo tree -p aspen-runtime-core`
- Status: pass
- Artifact: `openspec/changes/define-runtime-host-loading/evidence/runtime-core-dependency-tree.txt`

### `openspec validate define-runtime-host-loading --strict`
- Status: pass
- Artifact: `openspec/changes/define-runtime-host-loading/verification.md`

### `scripts/openspec-preflight.sh define-runtime-host-loading`
- Status: pass
- Artifact: `openspec/changes/define-runtime-host-loading/verification.md`

### `git diff --check`
- Status: pass
- Artifact: `openspec/changes/define-runtime-host-loading/verification.md`
