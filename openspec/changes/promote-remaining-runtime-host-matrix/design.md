## Context

Current Aspen runtime-host evidence includes product-path proofs for microVM CI, WASM, OCI lowering into an isolated WASM host, Hyperlight, and Hermit/Uhyve. The live harness suite set no longer contains the old `*-gap.ncl` files for those rows, but future rows may be introduced as metadata-only while the runtime-host taxonomy continues to evolve.

The durable invariant should be independent of the specific host: a row is readiness evidence only after Aspen starts or submits the unit through product runtime orchestration and records bounded operator-visible output or receipt data.

## Design

### Promotion package shape

Each future row promotion should start with an OpenSpec package that names:

- the current metadata-only row id or proposed row id;
- the host kind and artifact profile being promoted;
- the Aspen product path used for submission or reconciliation;
- the proof marker and receipt fields that distinguish real host execution;
- negative guardrails for model-only, package-only, admission-only, and direct-worker-only tests;
- harness metadata and readiness-doc updates required after proof passes.

### Product-path evidence

A promoted row must exercise Aspen-owned orchestration such as RPC/CLI/runtime reconciliation, `JobManager`/`WorkerPool`, or another product-facing runtime manager. Direct helper calls may exist as unit tests, but cannot be the row proof.

### Receipt boundary

Receipts and logs must include non-secret identifiers: host kind, artifact identity, runner identity, lifecycle status, bounded output summary, and proof marker. They must not include tokens, tickets, private keys, cluster cookies, connection strings, or raw secret values.

### Gap handling

Rows without product-path proof remain explicit gaps or future work. Metadata-only rows should be non-runnable inventory entries, should carry a reason, and should not be assigned proof levels such as `aspen-spawned-execution`.

## Risks

- **Overclaiming via reused tests**: mitigated by proof markers and negative guardrails that reject direct-worker/model-only evidence.
- **Stale docs vs harness**: mitigated by requiring readiness docs and generated inventory to move with each promotion.
- **Secret leakage in evidence**: mitigated by receipt redaction requirements and bounded diagnostics.
