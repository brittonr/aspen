## Why

Molten's long-running runtime has hot loops in the vat, dataspace, and Iroh paths where span-level `tracing` cannot answer function-level latency questions. Existing instrumentation requires in-process subscribers, so it perturbs the process it measures and cannot attach to a live node after startup.

flux-profiler (github.com/gattaca-com/flux, `crates/flux-profiler`, `Apache-2.0 AND MIT`) offers a different shape: `#[timed]` annotations write marks to per-thread shared-memory rings, and a separate CLI process attaches to a running node and exports a Perfetto/magic-trace trace. Enabled overhead is about 12 ns per frame, and the `disable-profiling` feature compiles every annotation out entirely. This fits development-time latency work on the runtime shell without changing production binaries.

## What Changes

- Add flux-profiler as a pinned git dependency, feature-gated and development-only, with the exact upstream revision recorded in the flake and vendor inputs. r[dev_profiling.dependency]
- Make `flux-profiler/disable-profiling` the default for release and production builds so annotated functions compile to plain function bodies. r[dev_profiling.build_gating]
- Restrict `#[timed]` and `enable_profiler` to the std runtime shell and its hot paths; pure cores and `no_std` crates stay unannotated because a mark write is an observable side effect. r[dev_profiling.placement]
- Support cross-process capture through the flux-profiler CLI in the dev shell, with bounded duration and memory, exported as `.fxt` for Perfetto or magic-trace. r[dev_profiling.capture]
- Support x86_64-linux only; other platforms build with profiling compiled out. r[dev_profiling.platform]
- Keep the optional `perf` and `alloc-profile` features off by default; `alloc-profile` chains behind an explicit workspace feature that swaps the global allocator. r[dev_profiling.optional_features]
- Treat every trace as a development observation, never as evidence: traces carry no Valence, Cairn, release-readiness, or determinism role. r[dev_profiling.non_claim]
- Add positive and negative build, placement, capture, and non-claim conformance. r[dev_profiling.verification]

## Impact

- **Runtime shell**: gains opt-in `#[timed]` annotations on selected hot functions and one `enable_profiler` call behind a profiling feature.
- **Pure cores**: unchanged. `molten-core` and `aspen-core` receive no dependency and no annotation.
- **Nix**: flake and vendor inputs gain a pinned fixed-output fetch for the flux git revision; the dev shell gains the `flux-profiler` CLI.
- **Configuration/docs**: gain a short operator note on enabling, attaching, and the non-claim boundary.
- **Compatibility**: default builds are bit-identical in behavior; every annotation vanishes under `disable-profiling`.
- **Claims**: a trace describes one development capture on one machine. It does not prove performance properties, satisfy release gates, or enter any evidence bundle.
