# Profiling Without Semantic Authority

Function traces can reveal where a development process spends time, but they are not part of Molten's deterministic decision semantics. This article explains the optional profiling surface and how to use its observations without promoting them into authority or canonical evidence. It assumes familiarity with Cargo features and local tracing tools. The [development profiling guide](../../development-function-profiling.md) is the governing capture and placement reference. Return to the [Technical companion](../README.md).

## Instrument the shell, not the law

The architectural boundary is not “timing is harmless.” Profiling introduces observation machinery associated with host execution. The governing guide confines annotations to the standard runtime shell and excludes profiler dependencies, annotations, shared-memory access, and startup calls from `molten-core` and `aspen-core`. A shell can measure a call into a pure core without moving that observation effect into the core.

This preserves an important distinction: an in-memory transition may be deterministic for explicit inputs even though two host executions take different amounts of time. Scheduler activity, surrounding I/O, compiler choices, and machine conditions belong to execution observations, not to the transition's semantic result. The [modularity inventory](../../modularity-boundaries.md) similarly separates planned effects from the adapters that execute them.

The inspected placement includes a timed CLI vat command, dataspace routing, and live Iroh transfer functions, as described by the governing guide. Such sites describe selected measured intervals. They do not imply complete coverage of every descendant operation or a transaction boundary around asynchronous work.

## Feature selection changes the observation apparatus

The [Cargo configuration](../../../Cargo.toml) pins optional `flux-profiler` to a reviewed commit and separates four features: `profiler`, `profiler-perf`, `profiler-alloc`, and `profiler-disabled`. The ordinary profiler feature enables the optional dependency. Hardware counters and allocation counting are additional choices, not implicit properties of every capture.

For `x86_64-linux`, the dependency uses its supported profiling configuration. Other targets enable the upstream `disable-profiling` feature. In the [startup implementation](../../../src/profiling.rs), `enable_development_profiler` calls the upstream startup function only when both the `profiler` feature and supported target conditions hold; otherwise its body is empty. The disabled feature supports compiling annotated sites to plain bodies, as the guide states.

`profiler-alloc` installs the upstream `CountingAllocator` around `std::alloc::System` through a global allocator declaration. Therefore an allocation-counting build is a specific experimental configuration, not evidence about the allocator behavior of an uninstrumented default build. The guide likewise requires host permission for hardware-counter access and treats missing permission as an error rather than silently omitting requested counters.

Feature unification deserves review when interpreting an experiment. The declared features form a set selected for the build, not mutually exclusive modes enforced by the local declarations. Record the actual selected configuration rather than infer it from one command fragment or the presence of a trace filename.

## A bounded probe is not a benchmark

The inspected [profiler probe](../../../examples/profilerprobe.rs) runs for a three-second host-time interval, repeatedly calls an annotated frame that performs a wrapping multiplication behind `black_box`, and sleeps for one millisecond between iterations. It explicitly permits ambient-clock use because it is a development-only bounded capture probe.

This is suitable for checking that instrumentation and capture can produce observable events. It is not a representative application workload or an allocation benchmark. The sleep is deliberate probe behavior, and its elapsed time cannot be interpreted as pure arithmetic cost.

The governing capture command is bounded:

```sh
flux-profiler --duration 2s --max-mem 64MB --out target/molten-development.fxt
```

That command is suggested verification, not an executed capture reported here. The guide describes running the probe in one shell and attaching from another, then opening the resulting file in Perfetto or magic-trace. A missed attachment interval and an uninstrumented build are distinct explanations for absent useful events; neither establishes that application work was free.

## Worked observation-to-hypothesis scenario

Imagine, illustratively, a local trace showing a long interval near a live Iroh transfer call. A reasonable engineering hypothesis is that transfer-related work warrants closer investigation. It is not yet a claim that transport is the bottleneck for all inputs, that a pure planner is slow, or that a release meets a latency target.

The next experiment would control the workload and relevant build configuration, distinguish waiting from computation where the instrumentation permits, and compare repeatable measurements. A later optimization still needs correctness evidence for its changed behavior. A shorter interval in one trace cannot justify removing an authority check, changing canonical receipt content, or weakening admission criteria.

This separation also prevents category errors in evidence handling. A local timing observation answers a development question; a canonical receipt records an operation under its own schema and identity rules. They are not interchangeable merely because both can be stored as files.

## Artifact-role checks and their limits

`admit_profiler_artifact` accepts only paths with extension `fxt` and only `ProfilerArtifactRole::DevelopmentObservation`. It rejects the Cairn receipt, determinism evidence, release-readiness, and Valence evidence roles. Embedded tests exercise both the accepted development role and denied evidence roles.

The function checks an extension and an enum value. It does not parse a trace, inspect its contents, discover every possible upload path, or automatically enforce an organization-wide artifact policy. Renaming another file to `.fxt` would not make its contents a valid capture. The implementation is a narrow role gate consistent with the broader governing prohibition, not a content-authentication mechanism.

## Verification and non-claims

Suggested review covers supported and disabled feature configurations, annotation placement, bounded capture, allocation configuration, and the artifact-role tests. No profiling run or build was executed for this documentation. Performance claims require repeatable benchmarks and tests, as the governing guide states.

A trace remains one machine-local observation. It grants no execution authority, is not deterministic evidence, does not certify a performance property, and is not a release-readiness input. Its legitimate value is diagnostic: form a precise hypothesis, design the next controlled experiment, and preserve the semantic boundary while improving the implementation.

## Sources

- [Development function profiling](../../development-function-profiling.md)
- [Modularity boundary inventory](../../modularity-boundaries.md)
- [Cargo features and target dependencies](../../../Cargo.toml)
- [Profiler startup, artifact-role checks, and tests](../../../src/profiling.rs)
- [Bounded profiler probe](../../../examples/profilerprobe.rs)
- [Technical companion](../README.md)
