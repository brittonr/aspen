## Context

`runtime-host-microvm-ci-vm` is now proven by a gated nested-KVM E2E check and documented in `docs/runtime-host-readiness.md`. The remaining metadata-only rows are WASM, OCI lowering, Hyperlight, and Hermit. WASM is the next best promotion target because it can be proven with a lighter host boundary than nested microVMs while still exercising real runtime-host semantics.

Current source anchors:

- `test-harness/suites/vm/runtime-host-wasm-gap.ncl` records the gap and states that plugin CLI install/reload is not proof.
- `crates/aspen-runtime-core/src/lib.rs` already models WASM profile admission, instance lifecycle, and redacted receipts.
- `openspec/specs/runtime-host-loading/spec.md` defines WASM host requirements and the runtime-host E2E matrix contract.

## Goals

- Replace the metadata-only WASM row with a runnable target only after the target starts Aspen and executes a WASM unit through product runtime plumbing.
- Keep proof semantics comparable to the microVM row: product-visible output/receipt evidence, explicit proof level, and secret-safe logs.
- Add cheap negative coverage ensuring direct model tests or plugin-management smokes cannot satisfy the row.

## Non-Goals

- This change does not require nested KVM.
- This change does not require OCI lowering into WASM.
- This change does not make all Aspen plugins production runtime services.
- This change does not assert that Hyperlight-WASM or Extism plumbing is the selected implementation; implementation may choose the existing runner seam that best matches Aspen runtime contracts.

## Decisions

### 1. Promote WASM before OCI/Hyperlight/Hermit

**Choice:** The next runnable runtime-host row SHALL be WASM.

**Rationale:** WASM has existing runtime-core contracts and should admit a bounded, deterministic fixture without VM hardware prerequisites. OCI lowering depends on a target host, while Hyperlight and Hermit have stronger runtime/tooling prerequisites.

### 2. Require product-path execution

**Choice:** The promoted row MUST start Aspen and drive the WASM unit through the runtime-host path via product RPC/CLI/orchestration, then assert receipt/output evidence.

**Rationale:** Runtime-core unit tests prove model logic, and plugin install/reload proves management plumbing, but neither proves Aspen-spawned runtime execution.

### 3. Keep the row cheap but explicit

**Choice:** The WASM E2E should be a normal runnable harness target when it has no host-specific hardware dependency, but it must remain separable from default local smoke checks until runtime stability and duration are known.

**Rationale:** The row should be cheaper than nested KVM, but product-path E2E can still depend on feature flags, fixture binaries, and runtime services that are not part of every developer's default loop.

### 4. Receipts are the acceptance artifact

**Choice:** Acceptance requires a secret-safe receipt or product-visible output that names module identity, runner/host identity, lifecycle state, and bounded output summary.

**Rationale:** Logs alone are weak evidence and can overclaim. Receipts align the WASM row with the operator-facing runtime-host readiness model.

## Risks / Trade-offs

- **Runner seam not yet wired to product runtime:** Keep this change active until implementation creates a real route from Aspen orchestration to WASM execution rather than reducing the requirement.
- **Plugin plumbing confusion:** Add negative tests/docs assertions that plugin install/reload and runtime-core direct calls do not promote the row.
- **Secret leakage:** Reuse runtime-core redaction helpers and add negative receipt checks before accepting logs or artifacts.
- **Check flakiness:** Start with a deterministic fixture module and bounded timeouts; only mark the harness row runnable after repeatable local evidence.

## Validation Plan

1. Add a tiny deterministic WASM fixture or generated module that emits a known output through declared host functions.
2. Add a product-path E2E target that starts Aspen with the WASM runtime feature set, submits/activates the fixture through the runtime path, and reads the resulting receipt/output.
3. Add negative coverage proving metadata-only/model-only/plugin-management paths do not count as `aspen-spawned-execution`.
4. Replace `runtime-host-wasm-gap` with a runnable harness manifest that sets `runtime_host.kind = "wasm"`, `proof_level = "aspen-spawned-execution"`, and a target command/build attr.
5. Regenerate/check `test-harness/generated/inventory.json`, run strict OpenSpec validation, run the new runnable target, and update runtime readiness docs with the accepted evidence.
