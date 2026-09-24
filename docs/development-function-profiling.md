# Development function profiling

r[impl dev_profiling.dependency]
Molten uses `flux-profiler` for optional function-level development traces.
The dependency is pinned to commit `2a1916465ae6649aebef3758233cfea98e5d33db`.
The Nix input and Cargo dependency use this same commit.

r[impl dev_profiling.platform]
Profiling is available only for `x86_64-linux` development builds.
Default builds do not enable the optional dependency.
Other targets compile each annotation out through the upstream `disable-profiling` feature.

## Build and capture

Enter the development shell and build the instrumented probe:

```sh
nix develop
cargo build --example profilerprobe --features profiler
```

Run the probe in one terminal:

```sh
cargo run --example profilerprobe --features profiler
```

r[impl dev_profiling.capture]
Attach from a second development shell.
Every documented capture has an explicit time or memory bound:

```sh
flux-profiler --duration 2s --max-mem 64MB --out target/molten-development.fxt
```

Open the `.fxt` file in Perfetto or magic-trace.
Each thread has a separate track.

r[impl dev_profiling.optional_features]
Use `profiler-perf` only when hardware counters are required.
The host must permit `rdpmc` access through its `perf_event_paranoid` policy.
A missing permission is an error, not an omitted counter.

Use `profiler-alloc` only when allocation counts are required.
This feature installs the upstream `CountingAllocator` for that build.
Default builds retain their normal allocator.

r[impl dev_profiling.build_gating]
Use `profiler-disabled` to compile annotated sites to plain function bodies.
This feature is suitable for explicit release-stripping checks.

## Placement boundary

r[impl dev_profiling.placement]
Annotations exist only in the standard runtime shell.
The initial sites cover the CLI vat command, dataspace routing, and live Iroh transfer calls.

`crates/molten-core` and `crates/aspen-core` must not contain profiler dependencies, annotations, shared-memory access, or startup calls.
A shell call site measures a pure-core operation without moving the side effect into that core.

## Claim boundary

r[impl dev_profiling.non_claim]
A trace is one machine-local development observation.
It is not deterministic evidence and does not prove a performance property.

Do not add `.fxt` files or profiler output to these surfaces:

- Valence evidence bundles
- Cairn receipts
- release-readiness inputs
- determinism claims

Use repeatable benchmarks and tests for performance claims.
Profiler observations can only guide local development decisions.
