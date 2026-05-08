## Context

Current proven runtime-host rows are Cloud Hypervisor microVM CI and product-path WASM jobs. Hyperlight remains metadata-only in `test-harness/suites/vm/runtime-host-hyperlight-gap.ncl` even though Aspen has `HyperlightWorker`, `vm_execute` payloads, node worker registration plumbing, and example guest material. Existing `vm_executor_test` coverage is construction/serialization/negative retrieval coverage; the ignored execution test does not launch a real guest from a stored artifact through job orchestration.

## Goals / Non-Goals

**Goals:**

- Specify the exact product-path proof needed before the Hyperlight row can be promoted.
- Keep proof markers operator-visible and secret-safe.
- Preserve anti-overclaiming for worker construction, payload serialization, example builds, and ignored/manual tests.

**Non-Goals:**

- Implementing the runnable Hyperlight target in this spec-foundation slice.
- Requiring nested-KVM execution in default/local checks.
- Expanding OCI or Hermit support labels.

## Decisions

### 1. Hyperlight promotion needs job-orchestration evidence

**Choice:** A promoted row must submit a blob-backed Hyperlight guest binary as a `vm_execute` job through Aspen job management and observe completion through Aspen-visible state/receipts.

**Rationale:** Direct `HyperlightWorker::execute` calls and worker construction tests bypass the product scheduling/orchestration path and are insufficient runtime-host evidence.

**Implementation:** The future runnable suite should mirror the WASM proof shape where practical: in-memory or local product services, blob-backed artifact, `JobManager`, `WorkerPool`, registered `HyperlightWorker`, terminal job status, attempts count, and a stable receipt marker such as `ASPEN_HYPERLIGHT_RUNTIME_HOST_EXECUTED`. If real Hyperlight requires KVM/runtime prerequisites, the row should be gated/opt-in rather than default-local.

### 2. Negative guardrails stay first-class

**Choice:** The implementation must include negative coverage proving existing model/serialization/build/ignored tests do not satisfy the row.

**Rationale:** The matrix is an operator contract. Metadata rows become dangerous if broad tests can accidentally masquerade as execution evidence.

**Implementation:** Use a distinct guard marker such as `ASPEN_HYPERLIGHT_RUNTIME_HOST_PRODUCT_PATH_GUARD` and assert invalid or missing Hyperlight artifacts reach the product path before failing.

### 3. Receipts must be secret-safe and bounded

**Choice:** Receipts/logs may include artifact hashes, runner identity, lifecycle state, bounded output, exit status, duration, and proof marker, but must not include raw tickets, cluster cookies, private keys, registry credentials, host paths, or full unbounded stdout/stderr.

**Rationale:** Runtime-host evidence is operator-facing and may be copied into docs or incident reports.

## Risks / Trade-offs

- **Runtime availability:** Real Hyperlight execution may require KVM or host support. Mitigation: make the target gated/opt-in and keep the row a gap until the target passes.
- **Fixture drift:** Example guest binaries can drift from current Hyperlight ABI. Mitigation: keep the fixture committed or deterministically built by the suite and assert module/artifact identity.
- **Overclaiming:** Build success or worker registration alone could look like readiness. Mitigation: require terminal job execution receipt markers and negative guardrails.
