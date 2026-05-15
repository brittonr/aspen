## Context

Aspen jobs and runtime-host proofs already produce receipts, but their executable identity is usually embedded in executor-specific payloads. Unison's content-addressed computation model suggests a common Aspen layer: immutable execution closures whose dependencies are fetched by hash and whose receipts cite the exact closure that ran.

## Goals / Non-Goals

**Goals:**
- Introduce a portable, deterministic closure manifest usable by multiple executors.
- Make closure identity BLAKE3-addressed and receipt-visible.
- Keep capability admission explicit and deny-by-default.
- Land one narrow executor adapter before broad migration.

**Non-Goals:**
- Replacing Cargo/Nix/Git or source-code storage.
- Automatically serializing arbitrary functions.
- Changing the Iroh-only control-plane rule.

## Decisions

### 1. Closure manifest as data, not executor code

**Choice:** Define a serialized `ExecutionClosureManifest` model containing hashes, schemas, runtime target, capability requirements, and provenance.

**Rationale:** The model can be validated, hashed, transferred, and tested independently from runtime execution.

**Alternative:** Add closure fields directly to each executor payload. Rejected because it preserves fragmentation and weakens receipts.

### 2. BLAKE3 closure hash over canonical manifest bytes

**Choice:** The closure hash is computed over canonical manifest serialization; dependency artifacts keep their own BLAKE3/blob identities.

**Rationale:** Aspen proof culture already treats BLAKE3 receipts as stronger than narrative.

**Alternative:** Use human names or job IDs as identity. Rejected because those are mutable operational handles.

### 3. Adapter-first migration

**Choice:** Implement one executor adapter first, preferably a cheap worker path with existing product-path tests such as WASM or shell.

**Rationale:** Avoid broad executor churn while proving the receipt and transfer contract.

## Risks / Trade-offs

**Manifest churn** → Version the schema and preserve compatibility tests.

**Overclaiming type safety** → Use explicit schema hashes and validation; do not claim compiler-level function serialization.

**Dependency fetch latency** → Cache dependencies by hash and record cache-hit/miss evidence in bounded diagnostics.
