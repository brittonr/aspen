# Validation evidence

## Scope

This change adds an opt-in development profiler to the standard Molten shell.
It does not add profiler effects to `molten-core` or `aspen-core`.
It does not promote profiler output into evidence or release decisions.

Base source commit: `d29f341860f95c6103c7056cea457e934e4cdb39`.

## Dependency identity

Cargo and Nix pin `gattaca-com/flux` commit `2a1916465ae6649aebef3758233cfea98e5d33db`.
The accepted upstream version is `0.1.3`.

The generated Nix source hash is `sha256-1ZBLORVsTtOXmOWdcfkQZJ/k3Ss+KJY3BMFBJwiSQDE=`.
SHA-256 is used because the Nix source-lock format defines this interoperability field.

Cargo generated the lockfile.
Nix generated the flake lock entry.
The repository-owned unit2nix tool regenerated both build plans.
No lock or generated plan was edited by hand.

The generated plans bind Cargo lock SHA-256 `790a476f6faa07c3e1e558517c8ba2dbca83b0f306bc071c204cc8d132071e71`.
The Molten Nix build plan selects `profiler-disabled` for release stripping.
Default Cargo builds do not select the optional dependency.

## Positive and negative checks

The pinned Nix `dev-function-profiling` check built the matching `flux-profiler` CLI.
Its help output exposes both `--duration` and `--max-mem`.

An enabled `x86_64-linux` probe published one thread ring.
A one-second, 64-MB-bounded capture produced an FXT trace containing `molten_profiler_probe_frame`.
The trace stayed under ignored `target/` storage and is not attached as evidence.

A default probe published no ring.
The CLI rejected its process as uninstrumented and created no trace.

A release probe built with `profiler-disabled` contained no `molten_profiler_probe_frame` symbol.
The `profiler-alloc` and `profiler-perf` feature compositions compiled successfully.

Positive and negative repository tests passed for revision pinning, pure-core placement, bounded commands, and evidence-role rejection.
The only source allowance is the development probe's explicit ambient-clock read.

## Validation

`cargo test -p molten` passed 1,387 tests across the library, binary, and integration targets.

Focused profiler boundary tests passed: six tests.
Focused artifact-role tests passed: three tests.

`cargo clippy -p molten --all-targets --all-features -- -D warnings` passed.
`cargo fmt --all --check` passed.

`cargo octet check` reported 5,856 existing warning-level findings.
This matches the base cohort count and adds no new finding.
The warning-only result is not strict Octet acceptance evidence.

`nix build .#checks.x86_64-linux.molten --no-link -L --option builders ""` passed.
The Nix nextest rail ran 1,387 tests with no failures or skips.
Its CI receipt is `blake3:1262c047a5f0bf13d3d702eef7302915066bb27e081ae75a8818e141d8783423`.

Strict Cairn validation passed before closeout.
Final gate receipts before sync:

- proposal: `f2b55ef17287c308b30a1b3d0cb6ee493dd966b94c1b1357ab848cfbc6859514`
- design: `027b9efa94ed0ab0e0a1f87ce7747fdf21ec08a39166e469bc41a8e11d63641d`
- tasks: `220423818b74081831623fa1e69c111d843f45677cc65f6511e7630607b5d125`

The sync dry-run passed with plan `8ae0af310fb26a9d151c634c1464e0d47fc446eeb63897f29d3763265f3e75d2`.
The executed sync added all requirements to the accepted development profiling specification.
Strict validation passed after sync.

The archive dry-run passed with plan `0195067b9fc8521810fd03a1ad28fa864273e4740e526e2d0d3f05fcd1277822`.
Archive execution moved the package to `2026-08-24-add-flux-profiler-dev-profiling`.
The archive receipt is `676e8c1cc8093d2c63239216576961ed6c610fb22ca1875a6ab1d991e56335d2`.
Strict validation passed after archive execution.

## Non-claims

A profiler trace is one machine-local development observation.
It does not prove latency, throughput, determinism, policy, authority, release readiness, or production behavior.
It has no Valence role and cannot satisfy a Cairn or release gate.
