# Wasm component performance evidence

Molten's Wasm component performance rail records host-scoped measurements
without changing component correctness, authority, or build boundaries. The
reviewed profile is authored in
[`wasm-component-performance/profile.ncl`](wasm-component-performance/profile.ncl)
and exported deterministically to
[`generated/profile.json`](wasm-component-performance/generated/profile.json).
Rust checks the export at startup; it does not evaluate Nickel at runtime.

## Sightglass boundary

The rail pins Bytecode Alliance Sightglass revision
`c18bbe75803a6a610f7ff3b15549c927c6e02667` and its raw measurement schema.
Sightglass measurements remain separated into compilation, instantiation, and
execution phases. Fast and deep lanes use distinct bounded process/iteration
plans and retain exact suite, workload, host-class, engine-cohort, resource, and
Mantle bundle identities. The exported refs are reviewed template-fixture
identities; an executable suite instance replaces bundle, workload, runner, and
engine artifact refs only with sorted BLAKE3 identities remeasured from the
actual Mantle materializations while retaining the reviewed lane configuration.

Sightglass's stock benchmark API is core-module oriented. Any benchmark engine
or wrapper that embeds a Molten component is therefore a Mantle-produced input
with its own exact bundle and engine-cohort identity; Molten does not build that
adapter, component, or engine. `run_sightglass_process` is a thin process shell
that invokes the pinned CLI shape and parses bounded raw JSON. Before spawning,
the outer compatibility shell opens the runner, engine, and benchmark with
no-follow semantics, bounds and rehashes each opened read-only regular file
against the admitted suite refs, and executes those same handles through Linux
`/proc/<pid>/fd/<fd>` locators. Unsupported hosts deny instead of falling back
to path-based execution. The shell owns the reviewed process count by running
single-process Sightglass subruns, aggregating them under one total runtime and
output bound, and never delegating deep-lane process fan-out to the child CLI.
Original executable, engine, and benchmark paths, stderr, raw operating-system
process IDs, and other host diagnostics never enter canonical receipts;
process IDs are normalized to bounded suite-local
ordinals before receipt construction.

The pinned source revision was also built as `sightglass-cli 0.1.0` with default
Callgrind support disabled, and its `benchmark --help` surface was smoke-checked
for the exact flags used by the shell. That diagnostic build is not a Mantle
artifact and cannot enter a production receipt. Full component measurements
remain conditional on separately admitted Mantle-produced runner, engine, and
core-adapter bundles; Molten does not substitute a locally built guest when
those bundles are absent.

## Comparison and claims

The pure comparison core rejects runs unless suite, source component, component
profile, performance profile, engine cohort, exact engine and runner artifacts,
runtime configuration, target, host class, measurement mechanism, resource
envelope, and phase/event groups match. Optimized artifacts may have different
output identities, but their source component identity and suite remain equal.

Samples use bounded integers. Means, a fixed 95% normal-approximation confidence
interval, candidate/baseline effect ratio, and a reviewed practical threshold
are computed with checked fixed-point integer arithmetic. This is deterministic
classification over supplied samples, not a claim that a noisy host produced a
representative population. Incompatible runs are reported and never ranked.

Canonical performance receipts carry full phase samples and bind the run,
optional comparison, optimization profile, Mantle, Valence, conformance, and
recorded-effect references. Comparison receipts also carry the exact peer run;
validation recomputes the comparison from both raw sample sets and rejects a
self-consistent fabricated class or missing peer. Contextual validation then
compares the receipt to independently derived run facts. Operator summaries are
explicitly non-normative.

## Build and optimization boundaries

Portable, Wizer-transformed, and precompiled artifacts must cross the exact
Mantle materialization boundary and are remeasured before use. The benchmark
shell contains no Wizer or precompile command. Wizer admission requires exact
source/output links, denied or deterministic virtual imports, repeated output
identity, pre/post receipts, and the semantic-equivalence non-claim.

Precompiled admission returns an `AdmittedPrecompiledComponent` token only after
source/output bytes, Wasmtime cohort, runtime configuration, component profile,
target, CPU features, build inputs, Mantle receipt, and Valence sidecars match.
This rail does not activate an unsafe deserialization path by itself.

Pooling allocation, copy-on-write heap images, `InstancePre`, compilation
strategy, concurrency, queue depth, memories, and tables are explicit profile
facts under hard caps. The reviewed cohort admits Cranelift and denies an
unreviewed Winch substitution. A conformance record must bind the exact
optimization configuration, identical input/output identities, matching
terminal classes, execution receipts, and recorded effects. Capacity exhaustion
returns start, backpressure, or deny; it never expands limits.

## Non-claims

Performance evidence is recorded-only. It does not prove behavioral
correctness, determinism beyond the cited conformance run, authority, security,
cross-machine performance, cross-runtime ranking, semantic equivalence, or
release eligibility. Sightglass itself explicitly scopes its suite to
Wasmtime/Cranelift comparisons rather than general runtime ranking; Molten keeps
that same boundary.
