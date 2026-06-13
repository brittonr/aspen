## Why

Molten's runtime contract is broader than ordinary Rust unit tests can cover. Deterministic replay, policy admission, SAM dataspace routing, Wasm/Steel adapters, blob-backed jobs, typed storage, receipts, transcripts, and operator dogfood all need one coherent testing rail that exercises the same canonical boundaries the runtime uses in production.

The harness must not become an ad hoc side channel. Test stimuli, observations, adapter fixtures, traces, receipts, and expected results need to travel over Molten's canonical Preserves communication boundary so tests validate the real envelope spine instead of bypassing it with Rust-only mocks or brittle text logs.

## What Changes

- Define a first-class Molten testing harness with suite, case, step, fixture, oracle, run, and report artifacts.
- Make determinism and replay core harness invariants, not optional add-ons: every evidence-bearing integration, transcript, property, chaos, dogfood, or CI run is deterministic by construction, replays from a recorded effect log, or is explicitly marked non-replayable and unsuitable as deterministic evidence.
- Add preflight guards before implementation: explicit harness privileges, hermetic deterministic inputs, versioned schemas, fail-closed evidence, no invisible fixture mutation, production/test separation, secret/capability hygiene, governed golden updates, bounded resources/logical time, canonical scheduler/liveness outcomes, adapter conformance gates, and replay eligibility gates.
- Add a required Preserves communication rail for harness/runtime interaction: test commands, actor stimuli, dataspace assertions, adapter fixtures, effect responses, observations, traces, receipts, diagnostics, and expected outputs are represented as canonical Preserves values or Molten envelopes.
- Require Blake3 hashes over canonical Preserves bytes for test identity, oracle matching, cache keys, replay logs, and report refs.
- Support fresh deterministic local runs by default with pinned artifacts, dependency closure, policy/schema refs, handler profile, logical clock, seed or recorded effect log, and resource budgets.
- Provide admitted fake/fixture adapters for clock, random, storage, blob, network, policy, Wasm, Steel, and external services.
- Integrate hand-written tests, executable transcripts, record/replay playback, deterministic chaos, Hegel property tests, Trellis predicate checks, and dogfood receipts under one evidence model.
- Add adapter conformance suites so Iroh, Redb, Wasmtime, Steel, blob/content, policy, storage, and network adapters prove they preserve the same Preserves/effect contracts.
- Add cross-actor-kind interoperability suites so native Rust, Steel, Wasm component, adapter-backed, and remote-proxy actors communicate only through Molten envelopes, Preserves dataspace assertions, admitted hostcalls, and canonical effect records.
- Add system-layer suites for Synit/SAM-like operating behavior: demand-driven services, dependency resolution, readiness/failure assertions, logical supervision, restart/shutdown, scoped service refs, auto-retraction, policy admission, and deterministic replay.
- Export minimal reproducibility bundles for failures: suite ref, artifacts, dependency closure, initial snapshot, policy/schema refs, profile, seed or effect log, traces, receipts, and first-divergence report.
- Treat counterexample shrinking, negative/security suites, upgrade replay, boundary coverage, deterministic multi-peer simulation, resource/performance regression, golden canonical traces, and flake prevention as harness rails.
- Emit canonical per-step traces, first-divergence diagnostics, receipts, and machine-readable reports for local developers and CI.

## Impact

This makes testing a runtime feature instead of a collection of scripts. The first milestone can add an in-process deterministic harness for two native actors, a Preserves-backed command/observation rail, a small fixture adapter set for clock/random/storage, canonical trace/oracle comparison, and a CLI entry point that emits a receipt-backed report.
