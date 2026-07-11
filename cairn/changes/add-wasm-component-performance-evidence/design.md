## Context

Wasmtime exposes distinct tuning mechanisms for compilation, instantiation, and execution. Sightglass likewise separates those phases and explicitly limits its claims to Wasmtime/Cranelift comparisons rather than general cross-engine benchmarking. Molten should preserve that discipline and bind every result to its deterministic component profile.

Precompiled Wasmtime artifacts are especially sensitive: deserialization assumes trusted bytes and a matching engine configuration. Performance work must therefore compose with Mantle build evidence and Valence identity rather than loading arbitrary `.cwasm` inputs.

## Decisions

### 1. Benchmark suites are immutable identified inputs

**Choice:** Define a typed Nickel benchmark-suite manifest covering exact component bytes, WIT/profile identity, workload inputs, warmup/sample plan, phase markers, host requirements, resource envelope, and non-claims. Each suite and fixture receives BLAKE3 identity.

**Rationale:** A benchmark name is not enough to compare results after workload, profile, or fixture drift.

### 2. Sightglass phase semantics are normative

**Choice:** Use Sightglass to measure compilation, instantiation, and execution separately for supported Wasmtime embeddings. Molten-owned wrappers may add receipt plumbing but must not collapse the phases into one latency number.

**Rationale:** An optimization can improve startup while degrading generated code, memory, or throughput. Phase separation keeps the trade-off visible.

### 3. Comparisons are bounded to compatible environments

**Choice:** A pure comparison core requires matching suite, component profile, engine cohort, target, host class, measurement mechanism, and resource envelope before computing effect sizes, confidence intervals, and regression classes. Incompatible runs are reported, not compared.

**Rationale:** Cross-host or cross-runtime numbers can be misleading even when the same Wasm bytes are used.

### 4. AOT artifacts require exact trust binding

**Choice:** Admit `.cwasm` only after verifying a Mantle build/precompile receipt and Valence sidecar that bind source component BLAKE3, output BLAKE3, Wasmtime cohort, full configuration identity, target, CPU features, WIT profile, and selected build inputs. Unknown or merely package-signed precompiled bytes deny before unsafe deserialization.

**Rationale:** Precompiled native code cannot be treated like validated portable Wasm bytes.

### 5. Runtime optimizations are separate profiles

**Choice:** Pooling allocation, copy-on-write heap images, `InstancePre`, compilation strategy, and concurrency capacity are named profile fields with explicit limits and backpressure. Each optimization profile must rerun deterministic conformance before performance evidence can be accepted.

**Rationale:** Performance knobs can alter allocation failure, memory reservation, scheduling, or startup behavior and therefore belong in runtime identity.

### 6. Wizer is a build transform with bounded evidence

**Choice:** Wizer execution occurs in the build shell with imports denied unless deterministic virtualization is explicitly declared. Repeated builds over identical admitted inputs must produce the same exact output identity before the artifact is eligible for runtime performance comparison. Receipts bind original module/component, initialization entrypoint, virtualized inputs, tool version, output, and non-claims.

**Rationale:** Preinitialization can embed host-derived state and is not semantic proof even when repeated bytes match.

### 7. Performance evidence stays recorded-only

**Choice:** Valence exports and Molten receipts classify benchmark and optimization results as recorded-only performance evidence. A Cairn or release policy may consume a threshold decision, but the benchmark itself cannot count as correctness, authority, or release proof.

## Functional core / imperative shell split

- **Pure core**: suite validation, compatibility checks, sample normalization, statistical comparison, regression classification, AOT manifest admission, optimization profile admission, and receipt construction.
- **Imperative shell**: build or load components, run Sightglass/Wasmtime, read host measurement facts, materialize Wizer/AOT outputs, enforce runtime backpressure, and write reports.

## Risks / Trade-offs

- Low-noise benchmarking can be expensive and host-specific. Keep a cheap smoke lane and a separate deep lane.
- Pooling can reserve substantial address space or memory. Limits and capacity must be explicit and tested under exhaustion.
- Repeated identical Wizer output detects some ambient drift but does not prove behavior preservation.
- Hard release thresholds can become flaky. Begin with recorded trend evidence and promote only after stable bounded fixtures exist.

## Non-Goals

- No cross-runtime benchmark ranking.
- No claim that faster execution is more correct, deterministic, secure, or release-ready.
- No acceptance of untrusted precompiled Wasmtime artifacts.
- No use of benchmark output as authority, policy, or provenance evidence.
