## Context

The workspace today uses the `tracing` ecosystem for span-level diagnostics. That answers "what happened" questions. It does not answer "which function consumed the time" questions on a live node without adding an in-process subscriber and restarting. Latency work on the vat, dataspace, and Iroh paths needs function-level frames with low overhead and post-hoc attach.

flux-profiler records open/close frames through a proc macro, writes them to per-thread shared-memory rings, and lets an external CLI drain the rings and export `.fxt`. Timestamps come from `rdtsc`, optional hardware counters from `rdpmc`, and optional allocation counts from a global-allocator wrapper. All of these are x86_64 and Linux centered.

## Decisions

### 1. Adopt the upstream crate; do not adapt the pattern

**Choice:** Depend on `flux-profiler` from `github.com/gattaca-com/flux` at one pinned revision. Do not vendor, fork, or reimplement the shared-memory ring pattern.

**Rationale:** The crate is young and single-vendor. A pinned external dependency keeps the upgrade and removal cost explicit. The pattern only becomes worth copying if a deterministic observation surface later needs it, which is out of scope here.

### 2. Pin the revision through the flake

**Choice:** The git revision is recorded once in the flake as a fixed-output fetch, and the Cargo vendor hash derives from it. The dev shell exposes the matching `flux-profiler` CLI built from the same revision.

**Rationale:** The crate is not published on crates.io. An unpinned git dependency would break reproducible Nix builds and make captures non-comparable across checkouts.

### 3. Profiling is opt-in per build, off by default in release

**Choice:** A workspace feature (for example `profiler`) enables `flux-profiler` in the runtime shell. Release and production builds always compose `flux-profiler/disable-profiling`, which collapses each `#[timed]` site to the plain function body with no atomic load.

**Rationale:** Annotations stay in the source permanently. Production binaries carry zero instrumentation, so there is no ambient overhead and no shared-memory surface in deployed nodes.

### 4. Annotate the shell, never the pure core

**Choice:** `#[timed]` and `enable_profiler` appear only in the std runtime shell and only on selected hot functions. `molten-core`, `aspen-core`, and every `no_std` crate receive no dependency and no annotation.

**Rationale:** Each mark is a shared-memory write, an observable side effect. Placing one inside a pure core would break the functional-core boundary that Octet policy and the workspace AGENTS.md enforce. Latency questions about a core are answered by timing its shell call site.

### 5. x86_64-linux only, other platforms compile it out

**Choice:** Profiling support targets x86_64-linux. Other targets resolve the dependency with `disable-profiling` so the source stays portable.

**Rationale:** Timestamps use `rdtsc`/`rdtscp` and hardware counters use `rdpmc`. Multi-socket TSC calibration is Linux-only upstream. Declaring one supported target keeps the claim honest.

### 6. Optional features stay opt-in and named

**Choice:** `perf` (hardware counters via `rdpmc`) and `alloc-profile` (per-frame allocation counts) are off by default. `alloc-profile` activates only through a workspace feature that also installs `CountingAllocator` as the global allocator for that build.

**Rationale:** `perf` adds about 50 ns per frame and needs `kernel.perf_event_paranoid <= 2`. `alloc-profile` replaces the process allocator. Both change the measured system, so each must be an explicit, reviewable choice.

### 7. Bounded captures by default

**Choice:** Documented capture invocations always set `--duration` or `--max-mem`. The CLI's 1GB default memory cap stays in place.

**Rationale:** An unbounded reader on a forgotten node grows without limit. Bounded captures match the workspace convention that every observation surface declares its limits.

### 8. Traces are observations, never evidence

**Choice:** `.fxt` traces and overhead numbers are development artifacts. They carry no Valence evidence role, no Cairn receipt linkage, no release-readiness weight, and no determinism claim. Structural guards keep trace files and profiler symbols out of evidence and release-policy paths.

**Rationale:** TSC timestamps are wall-clock and machine-local. Promoting a trace to evidence would silently strengthen claims the capture cannot support.

## Risks / Trade-offs

- Upstream is young and single-vendor. Pinning contains this; removal means deleting annotations, which compile out anyway under `disable-profiling`.
- A git dependency complicates the Nix vendor story. One fixed-output fetch in the flake keeps it reproducible.
- `#[timed]` on a hot leaf called millions of times per second can outrun the reader. The `--filter-short-frames` capture option and careful site selection mitigate this.
- Reader contention adds about 2 ns per frame while draining. Captures on production-like benchmarks must note whether a reader was attached.

## Non-Goals

- Continuous or production profiling of deployed nodes.
- eBPF, kernel, or ChaosControl observation surfaces; `chaoscontrol-trace` owns that domain.
- Replacing `tracing` spans, metrics export, or existing diagnostics.
- Recording traces as Valence evidence, Cairn receipts, or release-readiness input.
- Proving performance properties of any function or build.
