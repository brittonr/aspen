## Phase 1: Pinned dependency and feature gating

- [x] [serial] Add the flux git input to the flake at one exact revision with a fixed-output hash, and expose the `flux-profiler` CLI from the same revision in the dev shell. r[dev_profiling.dependency]
- [x] [serial] Add a workspace `profiler` feature that enables the `flux-profiler` dependency in the runtime shell crate only. r[dev_profiling.dependency] r[dev_profiling.placement]
- [x] [serial] Compose the `profiler` feature off and `flux-profiler/disable-profiling` on for every release and production build path. r[dev_profiling.build_gating]
- [x] [parallel] Add a negative vendor check proving an unpinned branch or path reference fails the Nix build. r[dev_profiling.dependency] r[dev_profiling.verification]
- [x] [parallel] Add a positive build check proving the pinned revision vendors reproducibly and the CLI runs. r[dev_profiling.verification]

## Phase 2: Instrumentation placement

- [x] [serial] Select the initial hot shell functions in the vat, dataspace, and Iroh paths and annotate them with `#[timed]`. r[dev_profiling.placement]
- [x] [serial] Add one `enable_profiler` call at shell startup behind the `profiler` feature, before any annotated function runs. r[dev_profiling.placement]
- [x] [parallel] Add a structural guard failing on any `flux_profiler` reference under `molten-core` or `aspen-core`. r[dev_profiling.placement] r[dev_profiling.verification]
- [x] [parallel] Rerun core purity checks and confirm no new shared-memory, environment, or clock effects in pure cores. r[dev_profiling.placement] r[dev_profiling.verification]

## Phase 3: Platform and optional features

- [x] [serial] Restrict profiling enablement to x86_64-linux and resolve `disable-profiling` on every other target. r[dev_profiling.platform]
- [x] [parallel] Add a positive x86_64-linux check that an enabled node publishes rings, and a negative check that another target builds the same source with profiling compiled out. r[dev_profiling.platform] r[dev_profiling.verification]
- [x] [parallel] Add named opt-in features for `perf` and `alloc-profile`, with `alloc-profile` installing `CountingAllocator` only for builds that select it. r[dev_profiling.optional_features]
- [x] [parallel] Add negative checks proving default builds keep the existing global allocator and omit hardware counters. r[dev_profiling.optional_features] r[dev_profiling.verification]

## Phase 4: Capture workflow and closeout

- [x] [serial] Document the enable-run-attach-open workflow with bounded `--duration` or `--max-mem` on every example invocation. r[dev_profiling.capture]
- [x] [parallel] Add a positive capture test: an instrumented example yields a `.fxt` trace containing the annotated frames. r[dev_profiling.capture] r[dev_profiling.verification]
- [x] [parallel] Add a negative capture test: a default build publishes no rings and the CLI finds no instrumented app. r[dev_profiling.build_gating] r[dev_profiling.verification]
- [x] [parallel] Add an evidence-role guard rejecting `.fxt` traces and profiler output in Valence bundles, Cairn receipts, and release-readiness inputs. r[dev_profiling.non_claim] r[dev_profiling.verification]
- [x] [serial] Run focused workspace tests, clippy, the structural guards, and `nix flake check` for the touched surface. r[dev_profiling.verification]
- [x] [serial] Run Cairn validation and the proposal, design, and tasks gates; sync and archive with capture evidence attached as development observations. r[dev_profiling.verification]
