# Dev Function Profiling Specification

## Purpose

Defines the `dev-function-profiling` capability.

## Requirements

### Requirement: flux-profiler is a pinned development-only dependency

r[dev_profiling.dependency] Molten MUST depend on `flux-profiler` from `github.com/gattaca-com/flux` at one exact pinned revision recorded in the flake as a fixed-output fetch, MUST gate the dependency behind an explicit opt-in workspace feature, and MUST NOT require it for default, release, or production builds.

#### Scenario: Pinned revision builds reproducibly

- GIVEN the flake records one exact flux git revision and its fixed-output hash
- WHEN the workspace builds with the profiling feature enabled
- THEN the vendored dependency MUST match the pinned revision
- AND the dev shell MUST expose a `flux-profiler` CLI built from the same revision.

#### Scenario: Unpinned or crates.io substitution is attempted

- GIVEN a dependency declaration references an unpinned branch, a local path, or a crates.io substitute
- WHEN the Nix vendor check runs
- THEN the build MUST fail until the revision and hash match the flake record.

### Requirement: Production builds compile profiling out

r[dev_profiling.build_gating] Release and production build compositions MUST enable `flux-profiler/disable-profiling` so every `#[timed]` site collapses to the plain function body, and the default workspace build MUST publish no shared-memory rings.

#### Scenario: Release build strips annotations

- GIVEN a release or production build composition
- WHEN the build runs
- THEN every `#[timed]` function MUST compile to its plain body with no atomic guard load
- AND the resulting binary MUST contain no profiler ring setup.

#### Scenario: Default run publishes nothing

- GIVEN a node binary built without the profiling feature
- WHEN the node starts and runs
- THEN it MUST NOT create shared-memory profiling rings
- AND a `flux-profiler` CLI attach MUST find no instrumented app for that process.

### Requirement: Annotations stay in the std shell

r[dev_profiling.placement] `#[timed]` and `enable_profiler` MUST appear only in the std runtime shell on selected hot functions, and pure cores and `no_std` crates (`molten-core`, `aspen-core`) MUST NOT depend on `flux-profiler` or contain profiler annotations, because a mark write is an observable side effect.

#### Scenario: Shell hot path is annotated

- GIVEN a selected hot function in the std runtime shell
- WHEN the profiling feature is enabled
- THEN the annotation MAY record open/close frames on every exit path
- AND `enable_profiler` MUST be called once at startup before any annotated function runs.

#### Scenario: Annotation appears in a pure core

- GIVEN source under `molten-core` or `aspen-core` references `flux_profiler`, `#[timed]`, or `enable_profiler`
- WHEN the structural placement guard runs
- THEN the check MUST fail
- AND core purity checks MUST continue to report no shared-memory or environment effects.

### Requirement: Capture is cross-process and bounded

r[dev_profiling.capture] Development captures MUST use the external `flux-profiler` CLI attaching to a running instrumented process, MUST bound every capture by `--duration` or `--max-mem`, and MUST export `.fxt` traces openable in Perfetto or magic-trace.

#### Scenario: Bounded capture produces a trace

- GIVEN an instrumented dev node runs with the profiling feature
- WHEN the CLI attaches with an explicit `--duration` or `--max-mem` bound
- THEN it MUST write a `.fxt` trace on stop, Ctrl-C, or app exit
- AND each thread MUST appear as its own track.

#### Scenario: Capture would run unbounded

- GIVEN a documented capture invocation omits both `--duration` and `--max-mem`
- WHEN documentation or tooling review runs
- THEN the invocation MUST be rejected or corrected to declare a bound.

### Requirement: Only x86_64-linux is a supported profiling target

r[dev_profiling.platform] Profiling enablement MUST target x86_64-linux, and every other target MUST resolve the dependency with `disable-profiling` so annotated source remains portable.

#### Scenario: Supported target enables profiling

- GIVEN an x86_64-linux build with the profiling feature
- WHEN the node starts with `enable_profiler`
- THEN it MUST publish per-thread rings under the declared app name.

#### Scenario: Unsupported target builds the same source

- GIVEN a non-x86_64 or non-Linux target
- WHEN the workspace builds that target
- THEN profiling MUST compile out completely
- AND the build MUST NOT require `rdtsc`, `rdpmc`, or shared-memory support.

### Requirement: Counter and allocator features are explicit opt-ins

r[dev_profiling.optional_features] The `perf` hardware-counter feature and the `alloc-profile` allocation-count feature MUST remain off by default, and `alloc-profile` MUST activate only through a named workspace feature that installs `CountingAllocator` as the global allocator for that build alone.

#### Scenario: Hardware counters requested explicitly

- GIVEN a build enables the workspace feature chaining to `flux-profiler/perf`
- WHEN the runtime lacks `kernel.perf_event_paranoid <= 2`
- THEN capture setup MUST report the missing permission rather than silently dropping counters.

#### Scenario: Default build keeps the plain allocator

- GIVEN a build without the allocation-profiling feature
- WHEN the global allocator is selected
- THEN it MUST remain the allocator the node already uses
- AND no `CountingAllocator` wrapper MAY be installed.

### Requirement: Traces carry no evidence or release role

r[dev_profiling.non_claim] Profiler traces, overhead numbers, and capture artifacts MUST be treated as development observations only, MUST NOT enter Valence evidence bundles, Cairn receipts, release-readiness inputs, or determinism claims, and MUST NOT be cited as proof of performance properties.

#### Scenario: Trace presented as evidence

- GIVEN a `.fxt` trace or profiler output is submitted to an evidence bundle, receipt, or release-readiness check
- WHEN evidence-role validation runs
- THEN the artifact MUST be rejected as a non-evidence development observation.

#### Scenario: Capture informs a development decision

- GIVEN a bounded capture on one development machine
- WHEN a developer compares two implementations
- THEN the trace MAY inform local optimization work
- AND any resulting claim MUST rest on benchmarks and tests, not on the trace alone.

### Requirement: Profiling adoption has positive and negative conformance

r[dev_profiling.verification] The adoption MUST include positive and negative checks for pinned-revision vendoring, feature gating, release stripping, shell-only placement, bounded capture, platform gating, allocator selection, and evidence-role rejection.

#### Scenario: Enabled build captures expected frames

- GIVEN an instrumented example or dev node with known annotated functions
- WHEN a bounded capture completes
- THEN the trace MUST contain frames for the annotated functions
- AND a disabled build of the same source MUST publish no rings.

#### Scenario: Guards reject boundary violations

- GIVEN a change introduces an unpinned dependency, a core annotation, an unbounded documented capture, or an evidence-role trace
- WHEN the corresponding guard runs
- THEN that guard MUST fail with a diagnostic naming the violated boundary.
