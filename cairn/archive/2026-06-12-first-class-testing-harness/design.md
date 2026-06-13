## Context

Molten already has separate requirements for deterministic playback, executable transcripts, operator dogfood, evaluation caching, resource governance, Preserves communication boundaries, and Hegel property tests. Those are necessary but not sufficient as a day-to-day testing experience. Developers and CI need one rail that can compose them without weakening Molten's core contract.

The testing harness is itself part of the runtime boundary. If tests inject state through private Rust structs, compare human logs as the only oracle, or mock adapters without preserving effect envelopes, they can pass while the real system violates policy, evidence, or replay laws. The harness therefore uses Molten's canonical Preserves envelope spine for all semantically relevant communication with the runtime.

## Goals

- Make tests content-addressed, reproducible artifacts with explicit dependency closure and policy/profile identity.
- Exercise the same envelope, dataspace, policy, effect, trace, receipt, and adapter boundaries used by production runtime paths.
- Provide deterministic local execution by default and record/replay for production-like incidents.
- Make first-divergence diagnostics a standard test outcome, not a special replay-only feature.
- Let CI and local developers run suites, inspect traces, export receipts, and attach reports to artifacts.
- Support small unit/model checks and larger integration/dogfood flows through one evidence format.

## Non-Goals

- Do not replace ordinary Rust unit tests for pure helper functions.
- Do not let tests bypass policy/evidence gates unless the bypass itself is represented as an explicit admitted test capability.
- Do not make terminal text snapshots the authoritative oracle when canonical records are available.
- Do not require live remote peers or production adapters for the first milestone.
- Do not claim compatibility with any external test framework's protocol; export adapters can be added later.

## Harness artifact model

A test suite artifact should describe:

- suite id, case ids, and step ids,
- artifact and dependency refs under test,
- initial runtime/store/dataspace fixture refs,
- schema refs, policy refs, and capability fixtures,
- handler profile id and config hash,
- deterministic seed or recorded effect-log ref,
- resource grants and backpressure expectations,
- expected traces, receipts, diagnostics, outputs, and final state hash,
- confidentiality/redaction metadata for report export.

A test run artifact should record:

- runner version and runtime/tool versions,
- resolved dependency closure hash,
- initial and final state hashes,
- per-step canonical command and observation refs,
- trace and receipt refs,
- first-divergence diagnostics if any,
- machine-readable status and failure classification,
- rendered developer output as a view over canonical data.

## Preserves communication rail

The harness/runtime boundary is a communication boundary. Semantically relevant harness messages MUST be canonical Preserves values or Molten envelopes. This includes:

- harness control commands: start runtime, install artifact, bind profile, run actor, advance logical time, stop runtime,
- actor stimuli: messages, assertions, retractions, Observe patterns, choreography endpoint inputs, job submissions,
- fixture and adapter traffic: effect requests/responses for clock, random, storage, blob, network, policy, Wasm, Steel, filesystem, process, and external services,
- observations: delivered envelopes, visible assertions, committed actions, adapter events, resource decisions,
- evidence: trace records, receipts, policy decisions, state hashes, snapshot refs,
- oracles: expected Preserves values, patterns, receipt predicates, trace predicates, divergence expectations,
- diagnostics and reports: first-divergence records, rendered diffs, redaction markers, export metadata.

Blake3 hashes are computed over canonical Preserves bytes. Large payloads are referenced through content refs with Preserves metadata. Developer-friendly text, JSON, TAP, JUnit, or markdown reports may be rendered from canonical records, but they are not the primary evidence or oracle.

The rail should expose typed Rust DTOs for ergonomics, but those DTOs must round-trip through the canonical Preserves representation at the boundary. Private Rust-only shortcuts may exist inside a pure unit test, but they do not satisfy integration, transcript, replay, or dogfood harness requirements.

## Execution modes

Initial modes:

- `fresh-local`: default deterministic in-process runtime with fresh fixture store and dataspace.
- `replay`: injects recorded effect responses and denies live external side effects.
- `record`: permits admitted real adapters and records canonical effect responses for later replay.
- `chaos`: deterministic seeded failures, delays, drops, reorders, partitions, and resource pressure.
- `transcript`: runs executable transcript stanzas as harness steps.
- `property`: executes Hegel generated cases and records generated inputs/counterexamples as Preserves artifacts.
- `dogfood`: runs the operator confidence workflow and emits final receipts.

All modes identify the handler profile and config hash. Modes that can perform external side effects require explicit policy admission.

## Fixture adapters

The harness should provide admitted fixture adapters for:

- logical clock and seeded random,
- in-memory and fixture-backed typed storage,
- local chunk/blob store and content-ref verification,
- local dataspace and actor scheduler,
- policy decisions and denial fixtures,
- fake remote peer/network events,
- Wasm hostcall fixtures and fuel budgets,
- Steel trusted-callable fixtures,
- resource grants and backpressure decisions.

Fixture adapters still communicate through effect request/response records. A fake adapter is not allowed to mutate runtime state invisibly.

## Oracles and matching

Expected outcomes should support:

- exact canonical Preserves value equality,
- Preserves pattern matching with deterministic binding order,
- trace and receipt predicate matching,
- expected denials, expected failures, and known bugs,
- final state hash and selected snapshot comparison,
- expected absence of side effects,
- first-divergence kind and boundary matching.

Text diffs are useful renderings. Canonical diffs and hashes are the stable evidence.

## Determinism and replay

Determinism and replay are core harness invariants. Any harness run that is used as integration evidence, transcript evidence, property-test evidence, dogfood evidence, CI evidence, or admission evidence must declare one of these statuses:

- deterministic: all nondeterminism is supplied by pinned deterministic handlers, seed/config, fixture state, and logical scheduling;
- replay: all external observations are injected from a recorded effect log and checked for request/response identity;
- record: admitted real adapters may run, but every external observation and effect response needed for replay is recorded canonically;
- non-replayable: exploratory only, not acceptable as deterministic evidence or an admission gate result.

Harness identity includes the deterministic inputs from the playback law: artifacts, dependency closure, initial state, schema refs, policy refs and capability state, handler profile, seed or recorded effect log, runtime/tool versions, and harness version. Deterministic modes must produce stable canonical trace/report refs for the same identity.

On divergence, the harness stops at the first semantic boundary it can identify and reports scheduler, input, effect request/response, policy decision, action, receipt, trace, output, or state mismatch with expected/actual canonical hashes and a redacted rendered diff where policy allows.

## Preflight implementation guards

Before implementation work treats the harness as evidence-bearing infrastructure, these guards apply:

### Harness privilege boundary

The harness has only explicit test capabilities. It must not call private runtime internals, mutate stores directly, bypass policy, or synthesize authority through helper APIs. Any intentional test-only bypass is itself an admitted capability with trace and receipt evidence.

### Hermetic deterministic inputs

Deterministic runs do not read ambient filesystem, environment, network, wall-clock, entropy, process state, or OS thread scheduling. Inputs come from declared fixtures, artifacts, seeds, logical clocks, deterministic profiles, or recorded effect logs.

### Schema and version discipline

Harness command, observation, effect, oracle, report, and repro-bundle schemas are versioned from the first milestone. Reports identify runner, runtime, tool, schema, policy, and artifact versions. Old reports must replay, migrate with receipts, or fail with compatibility diagnostics.

### Fail-closed evidence

Missing trace records, receipts, state hashes, effect records, profile identity, or replay identity are failures for evidence-bearing runs. The harness must never treat absent evidence as success.

### No invisible mutation

Fixture, fake, mock, simulator, chaos, and debug adapters cannot mutate semantic runtime state except through canonical effect records, committed actions, or explicit admitted test capabilities.

### Production separation

Test-only APIs, fixtures, bypass capabilities, debug hooks, and exploratory profiles are not available in production profiles unless explicitly admitted for record, replay, or debug use and recorded as evidence.

### Secret and capability hygiene

Reports and repro bundles are policy-gated. Capabilities, tokens, secret fixtures, external observations, and confidential trace fields are redacted or encrypted by default, with reveal requiring explicit authority.

### Golden update governance

Golden traces, receipts, state hashes, snapshots, and reports cannot be silently rewritten. Updates require review or policy authority, old/new refs, reason class, migration notes where applicable, and receipts.

### Resource and logical-time bounds

Every harness run declares bounds for turns, scheduler steps, logical time, effect calls, trace bytes, mailbox depth, assertion count, storage/blob/network bytes, Wasm fuel, Steel/native checkpoints, and job-stage resources. Exhaustion is a deterministic failure with evidence.

### Scheduler and liveness outcomes

Scheduler ordering is total and canonical. Quiescence, pending work, timeout, deadlock, starvation, cancellation, and supervisor escalation are explicit harness outcomes, not ambiguous wall-clock failures.

### Adapter contract gates

Adapters are not accepted into evidence-bearing profiles until they pass conformance suites against the Preserves/effect boundary for admitted requests, denials, failures, traces, receipts, resource decisions, and replay.

### Replay eligibility gates

CI, release, upgrade, admission, and policy evidence gates accept only deterministic, replayed, or recorded-for-replay runs. Exploratory or non-replayable runs may be useful diagnostics, but they cannot satisfy a deterministic gate.

## Additional harness rails

### Adapter conformance

Each runtime adapter should have conformance suites that exercise the same public Preserves/effect contract. Iroh, Redb, Wasmtime/WASI, Steel, blob/chunk stores, typed storage, policy, resource, and fake network adapters should prove that requests, responses, denials, receipts, traces, budgets, and state changes are canonical and replayable. Adapter-private APIs may be unit-tested separately, but adapter acceptance uses the harness rail.

### Actor-kind interoperability

Actor kind is an execution-adapter detail, not a communication semantic. Native Rust actors, Steel trusted-orchestration actors, Wasm component actors, adapter-backed actors, and remote proxy actors should interoperate through the same Molten envelope, Preserves dataspace assertion/retraction, Observe, hostcall, policy, effect, trace, and receipt boundaries. Steel must use public runtime APIs rather than mutating runtime internals; Wasm must use admitted hostcalls and deny-by-default WASI; adapter-backed actors must expose their observations as canonical effect records. Cross-kind suites should prove native-to-Wasm, Wasm-to-Steel, Steel-orchestrated spawning, adapter-backed responses, policy denial, and replay stability.

### System-layer suites

Molten may become a Synit/SAM-like system layer, but the harness must test Molten's own semantics rather than asserting Synit compatibility. System-layer suites should model services and system resources as dataspace facts: demand assertions, dependency assertions, readiness/failure/completion assertions, exposed service refs, scoped capabilities, and lifecycle state. The harness should prove demand-driven startup and shutdown, dependency gating, auto-retraction on crash/revocation/session close, logical supervision independent of OS parentage, restart/degrade policies, capability-scoped service refs, and deterministic replay of system behavior. Services may be implemented by native, Steel, Wasm, adapter-backed, or remote-proxy actors, but all service communication still uses the Preserves rail and policy/evidence gates.

### Reproducibility bundles

Every deterministic or recorded failure should be exportable as a minimal repro bundle containing the suite/case/step refs, artifact dependency closure, initial snapshot or fixture refs, schema/policy refs, handler profile, seed or effect-log segment, relevant traces and receipts, final or divergent state hashes, and first-divergence diagnostic. Repro bundles are content-addressed and policy-redacted so a developer or CI worker can rerun the same failure without ambient state.

### Counterexample shrinking

Property-test failures should not only report generated input. The harness records the generation seed, shrink path, final shrunk Preserves fixture, expected oracle, first-divergence report, and replay identity. Shrunk fixtures become ordinary test cases and transcript snippets where useful.

### Negative and security suites

Denied behavior is first-class. Suites should cover missing and revoked capabilities, malformed envelopes, invalid Preserves values, noncanonical encodings, tampered content refs, invalid receipts, policy denials, resource exhaustion, replay-protection failures, redaction leaks, confused-deputy attempts, and unauthorized report export. Passing means the runtime denied the action before side effects and emitted evidence.

### Upgrade and migration replay

The harness should run old canonical traces, reports, snapshots, schemas, policies, and artifacts against new runtime versions. Compatible changes must replay or migrate with receipts. Incompatible changes must produce explicit migration/compatibility diagnostics rather than silent trace drift.

### Boundary coverage

Coverage is tracked by runtime boundary, not only by source line. Reports should identify which envelope routes, dataspace semantics, policy gates, effect handlers, receipts, traces, storage paths, resource decisions, replay branches, adapter boundaries, and confidentiality paths were exercised.

### Deterministic multi-peer simulation

The harness should model multiple Molten peers in one deterministic process or simulation. Peer delivery, partitions, drops, reorders, reconnects, clock behavior, resource limits, and gossip/doc/blob observations are driven by seeded profiles or recorded logs. Multi-peer simulation validates remote behavior without requiring live production peers.

### Resource and performance regression

Suites may assert budgets over turns, scheduler steps, mailbox depth, assertion count, effect calls, blob/storage/network bytes, trace bytes, Wasm fuel, Steel/native operation checkpoints, and job-stage resources. Wall-clock performance can be reported as advisory metadata, but deterministic budget regressions are the normative gate.

### Golden canonical traces

Important runtime stories should have versioned golden trace/receipt/state-hash artifacts. Golden updates require explicit review, migration notes, and receipts identifying whether the change is intentional, schema-driven, policy-driven, or a bug fix.

### Flake prevention

CI evidence gates reject flaky tests by construction. A test is deterministic, replayed, recorded for replay, or exploratory/non-replayable. Exploratory runs may inform debugging but cannot satisfy deterministic evidence, admission, release, or upgrade gates.

## CI and CLI surface

Initial CLI shape may include:

```text
molten test list
molten test run <suite-ref-or-path>
molten test run <suite-ref-or-path> --profile local --seed <seed>
molten test replay <run-ref-or-path>
molten test report show <run-id>
molten test report export <run-id>
```

Reports are stored as canonical artifacts with optional rendered views. CI exporters can be added without changing the harness evidence model.

## Policy and confidentiality

Test suites and reports may contain secrets, capabilities, external observations, or exploit reproductions. Running and exporting them is policy-gated. Secret fields use redaction markers or encrypted refs. Replay must not grant authority outside the recorded scope.

## Integration points

- Runtime spine: validates the Preserves envelope and pure-core boundaries.
- SAM runtime: tests turn semantics, Observe, assertion lifetimes, attenuation, and service dependency assertions.
- Deterministic playback: shares scheduler, handler profiles, effect logs, snapshots, state hashes, and divergence diagnostics.
- Executable transcripts: transcript stanzas become harness steps and transcript-run receipts become harness receipts.
- Evaluation cache: deterministic harness runs can be memoized by canonical identity.
- Resource governance: harness profiles set bounded budgets and assert backpressure decisions.
- Dogfood receipts: local dogfood is a named harness suite with operator-visible receipts.
- Trellis predicates: bounded protocol/runtime predicates can run as harness checks.
- Hegel: generated cases and shrunk counterexamples are recorded as canonical Preserves fixtures.

## Open Questions

- What is the smallest stable Preserves schema for the first harness command and observation envelopes?
- Should the first CLI accept file paths only, artifact refs only, or both?
- Which report store lands first: Redb receipt index, content-addressed chunk store, or both?
- How much compatibility should exporters provide for JUnit/TAP without making them normative?
