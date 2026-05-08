## Context

Current proven runtime-host rows are Cloud Hypervisor microVM CI, product-path WASM jobs, and gated product-path Hyperlight jobs. OCI lowering remains metadata-only in `test-harness/suites/vm/runtime-host-oci-lowering-gap.ncl`. The existing `aspen-runtime-core` OCI surface is intentionally data/admission oriented: it models source OCI identity, selected isolated lowering target, derived artifact identities, provenance, diagnostics, and secret-safe receipts without actually pulling an image or launching a runtime host.

That model is necessary but insufficient for runtime-host matrix promotion. A promoted OCI row must prove the packaging-to-host bridge: immutable OCI input is resolved/lowered into an isolated artifact, the derived artifact runs through Aspen orchestration, and receipt evidence links the source OCI digest to the isolated host execution.

## Goals / Non-Goals

**Goals:**

- Specify the exact product-path proof needed before the OCI lowering row can be promoted.
- Keep source OCI identity, derived artifact identity, selected target host, execution marker, and receipt boundary auditable.
- Preserve anti-overclaiming for runtime-core model tests, registry/image metadata checks, package builds, and raw container/dev-only smokes.

**Non-Goals:**

- Implementing the runnable OCI-lowering target in this spec-foundation slice.
- Making raw host-container execution a production isolation boundary.
- Promoting Hermit or changing the already-proven microVM/WASM/Hyperlight claims.

## Decisions

### 1. OCI proof must include both lowering and isolated execution

**Choice:** A promoted row must start from an immutable `sha256:` OCI image digest or equivalent content-addressed OCI artifact, lower it into a declared isolated target artifact, then submit the derived artifact through an Aspen product path for a supported target host.

**Rationale:** OCI is not itself the runtime host in Aspen's production contract. Lowering evidence that stops at a plan/receipt model proves admission only; execution evidence that does not retain the source OCI identity proves only the target host. The row needs both.

**Implementation:** The future runnable suite should use a deterministic fixture OCI artifact and a deterministic lowering path into the cheapest compatible proven host. WASM is likely the lowest-cost default target if the fixture can be represented as an OCI-packaged WASM component; Hyperlight or microVM may be used when their gated prerequisites are explicitly declared.

### 2. Raw containers remain negative evidence

**Choice:** Raw Podman/Docker-style host-container execution must be rejected or labeled dev/unsafe and must not satisfy `aspen-spawned-execution` for the OCI lowering row.

**Rationale:** The row is about OCI artifact lowering into an isolated Aspen host, not ordinary container runtime readiness.

**Implementation:** Use a distinct guard marker such as `ASPEN_OCI_LOWERING_RUNTIME_HOST_PRODUCT_PATH_GUARD` and assert model-only plans, raw-container declarations, missing derived artifacts, or unlowered mutable tags cannot be accepted as the promoted row.

### 3. Receipts must link identities without leaking secrets

**Choice:** Receipts/logs may include source OCI digest, selected target host, derived artifact hash, runner identity, lifecycle state, bounded output, exit status, duration, and proof marker, but must not include registry credentials, raw environment secrets, mutable tags as durable identity, ambient host paths, tokens, connection strings, or private material.

**Rationale:** OCI evidence often involves registry and environment material; runtime-host promotion receipts must be safe to publish or paste into operator docs.

## Risks / Trade-offs

- **Fixture complexity:** Building a real OCI artifact and deterministic lowering path can sprawl. Mitigation: pick the smallest compatible proven host target and commit or generate a tiny fixture with stable hashes.
- **Target-host conflation:** A passing WASM/Hyperlight/microVM test could be misread as OCI proof. Mitigation: require source OCI identity plus lowering provenance in the same receipt.
- **Container overclaiming:** Convenient raw-container smokes may look like OCI execution. Mitigation: keep raw containers negative/dev-only and enforce guard markers.
