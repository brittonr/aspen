## Context

`src/wasm/performance/` owns the reviewed optimization profile model and its
admission (`OptimizationProfile` with `pooling_allocator`,
`copy_on_write_heap_images`, `instance_pre`, reviewed ids, and capacity bounds),
plus a Sightglass comparison rail for cross-runtime benchmarking. Nothing applies
those profile knobs to a local `wasmtime::Config` for a measurement, so the
profiles had never been compared on one workload from in-repo source.

Issue #41's first two rounds measured them from a scratch worktree. Both found
the same shape: compute-bound work is flat across profiles, and instance churn is
dominated by `pooling`.

## Decisions

### 1. The measurement is a diagnostic example, not a runtime path

**Choice:** Ship `examples/wasm-optimization-profiles.rs`, built with
`--release`, taking a workload name and a sample count.

**Rationale:** The example needs host time, so it belongs outside the pure cores;
the repository already uses `examples/` for development-only diagnostic tools
(see `examples/profilerprobe.rs`) with a stated `ambient_clock` reason. It
registers no capability and is not a runtime backend.

### 2. Two workloads, each mapping to one profile question

**Choice:** `loop` (one instance per sample, bounded integer loop over a 64 KiB
page) and `short-lived` (500 sequential instance lifecycles per sample, trivial
call). Each profile gets a fresh `Engine`, one warm-up sample, then the measured
samples; compile and instantiate+execute are timed separately.

**Rationale:** `loop` asks whether per-call work is sensitive to allocation
policy; `short-lived` asks whether instance churn is. The measured answers were
"no" and "pooling, by about 3x", which is exactly the distinction the selector
proposal needed.

### 3. Report samples, never a verdict

**Choice:** Emit `median`, `p90` and `max` microseconds per phase plus the
workload name, sample count and instances per sample. No pass/fail, no
recommendation, no receipt.

**Rationale:** A percentile is an observation under one host and build profile.
Turning it into a recommendation needs a threshold and a consumer policy, which
this change does not own; the issue thread records the interpretation.

### 4. The first artifact is a measurement, not a selector

**Choice:** Do not implement a learned or static selector here. Record the
per-shape numbers and let a later change decide.

**Rationale:** Both rounds show one dominant profile per workload shape, with no
per-request trade-off inside a process. A selector would add a model and an
artifact for a decision two static rules already answer.

## Functional core / imperative shell split

- **Pure:** percentile rounding and sample aggregation, which are unit tested.
- **Imperative shell:** engine construction, module compilation, instantiation,
  host-time sampling and printing.

## Risks / Trade-offs

- Host-time numbers are environment-specific: the example states the build
  profile and the workload, and the evidence record binds the commit and image.
- The pooling allocator's defaults are environment-sized; the example pins
  `total_memories`/`total_tables` and a 1 MiB memory ceiling so the comparison is
  reproducible.
- The example is not a benchmark receipt and must not be wired into gate logic.
