# Reviewed benchmark phase-group preflight verification

## Isolation and failing-before proof

The clean isolated branch `change/preflight-performance-phase-count-20260929` starts at canonical Aspen `81baf80cb667a22277be22c140d34829e5c7da7f`. The clean source entry points are in `src/wasm/performance/parts/comparison/p000/body.rs`; the detached composition has equivalent integrated functions in `src/wasm/performance/comparison.rs`, with the same comparison regression file. The supported profile validates its suites' phases against the three-entry `PerformancePhase::ALL` / `PERFORMANCE_PHASE_COUNT` contract. Internal producer callsites are test fixtures; receipt and comparison logic consume validator results, and no internal production CLI constructs a benchmark run.

Before editing either production function, `CARGO_TARGET_DIR=/home/brittonr/.cargo-target/aspen-performance-phase-owner-20260929 nix develop --command cargo test -p molten --lib performance_phase_count_preflight_ -- --test-threads=1` failed both new tests. The fourth producer group returned only `benchmark phase/event groups must be sorted and unique`, after normalization; the fourth forged validator group returned the duplicate-group error plus `benchmark run identity does not match its canonical samples`. Both tests' exact-three, admitted-invalid and duplicate assertions had passed before their fourth-group failures; the producer's existing workload-precedence assertion had also passed.

## Corrected public behavior

The producer checks `input.phases.len() > input.suite.phases.len()` after its existing profile, validated suite and run-input checks, but before sorting/normalization or per-phase diagnostics. The validator checks `run.phases.len() > PERFORMANCE_PHASE_COUNT` at function entry, before formatting malformed-ref diagnostics, scanning phase/sample groups or recomputing run identity. Both return one `PerformanceDenial` blocker: `benchmark run exceeds reviewed suite phase count`. This reuses the reviewed three-phase contract, not an arbitrary new cap. At or below three entries, canonical ordering, malformed group errors, duplicate-group diagnostics, missing producer phase errors and empty validator run errors retain their previous behavior.

After the fix, both focused tests passed **2/2** in each tree; each `wasm_performance::tests::` filter passed **17/17**. After adding explicit two-group/empty-run assertions, the focused two-test filter passed again in both trees. A separately compiled disposable package used the public `molten::wasm_performance` facade, admitted minimal portable component bytes through `verify_performance_materialization`, built and validated an unordered exact-three run, observed admitted duplicate diagnostics and both four-group denials. Its stdout was `performance public API: three normalized and validated; admitted duplicates retain errors; four producer/validator groups denied`; its temporary source was removed.

## Scoped checks and residual static findings

- `nix develop --command cargo fmt --package molten --check` passed both trees after the final test assertions.
- Owner `cargo clippy -p molten --all-targets -- -D warnings` and composition `cargo clippy -p molten --lib -- -D warnings` passed using their separate target directories.
- Isolated, nonincremental `cargo octet check --artifact-dir /tmp/molten-performance-phase-owner-lib-20260929 -- -p molten --lib` reported **835 warning-only findings**, zero errors/autofixable; its split comparison `p000` has no `unbounded_collection_growth` row.
- Composed, nonincremental `cargo octet check --artifact-dir /tmp/molten-performance-phase-composition-lib-20260929 -- -p molten --lib` reported **1,493 warning-only findings**, zero errors/autofixable. Its static `unbounded_collection_growth` findings still point to `src/wasm/performance/comparison.rs:74,268,280,354` despite the runtime entry guards. The guarded count repair does not clear these static rows or imply full strict Octet success.
- `nix develop --command cairn validate --root . --policy cairn-policy/generated/cairn-policy.json --strict` returned empty issue lists and `valid: true` for **53** isolated and **67** composed change packages. This is structural validation, not release acceptance.

No accepted spec, new arbitrary phase cap, waiver, policy, baseline, pin or retry of the unchanged known-failing full strict Octet gate was introduced.
