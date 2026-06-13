## Phase 0: Preflight guards

- [x] [serial] r[molten.testing.preflight_guards] Define preflight implementation guards for harness privilege boundaries, hermetic inputs, schema/version discipline, fail-closed evidence, no invisible mutation, production/test separation, secret hygiene, golden update governance, resource/logical-time bounds, scheduler/liveness outcomes, adapter contract gates, and replay eligibility gates.
- [x] [serial] r[molten.testing.preflight_guards.privilege_boundary] Ensure test-only bypasses are explicit admitted capabilities with trace and receipt evidence rather than private runtime backdoors.
- [x] [serial] r[molten.testing.preflight_guards.hermeticity] Deny ambient filesystem, environment, network, wall-clock, entropy, process state, and OS scheduling inputs in deterministic harness modes.
- [x] [parallel] r[molten.testing.preflight_guards.fail_closed] Fail evidence-bearing runs when required traces, receipts, state hashes, effect records, profile identity, or replay identity are missing.
- [x] [parallel] r[molten.testing.preflight_guards.production_separation] Keep test-only APIs, fixtures, bypass capabilities, debug hooks, and exploratory profiles out of production profiles unless explicitly admitted and evidenced.

## Phase 1: Harness model and Preserves rail

- [x] [serial] r[molten.testing.harness_artifacts] Define canonical suite, case, step, fixture, oracle, run, and report artifacts with dependency closure, policy refs, handler profile, seed/log refs, and state hashes.
- [x] [serial] r[molten.testing.determinism_replay_core] Make every evidence-bearing harness run declare deterministic/replay status: deterministic profile with seed/config, replay profile with effect log, record profile producing replay log, or non-replayable exploratory status excluded from deterministic evidence.
- [x] [serial] r[molten.testing.preserves_comm_rail] Require harness control, stimuli, adapter fixtures, observations, traces, receipts, diagnostics, oracles, and reports to cross the harness/runtime boundary as canonical Preserves values or Molten envelopes.
- [x] [serial] r[molten.testing.boundary_hashes] Compute harness identity, oracle, cache, replay-log, and report refs from Blake3 hashes over canonical Preserves bytes and content refs.
- [x] [parallel] r[molten.testing.preserves_comm_rail.rendering] Document that text, markdown, JSON, JUnit, TAP, and terminal output are rendered views over canonical records, not primary evidence or oracles.

## Phase 2: Deterministic local runner

- [x] [serial] r[molten.testing.fresh_local_runner] Implement a fresh deterministic local runner that starts an in-process runtime, installs artifacts, binds a handler profile, executes steps, and cleans up fixture state by default.
- [x] [serial] r[molten.testing.fresh_local_runner.no_ambient_state] Add the first two-native-actor harness suite covering send, observe, assert, retract, commit, rollback, trace, and final state hash.
- [x] [serial] r[molten.testing.fixture_adapters] Add admitted fixture adapters for logical clock, seeded random, in-memory storage, local content refs, policy decisions, and resource budgets.
- [x] [parallel] r[molten.testing.cli_surface] Add initial CLI commands for listing suites, running suites, replaying runs, showing reports, and exporting reports.

## Phase 3: Oracles, diagnostics, and evidence

- [x] [serial] r[molten.testing.canonical_oracles] Compare exact Preserves values, Preserves patterns, trace predicates, receipt predicates, expected denials, final state hashes, and expected absence of side effects.
- [x] [serial] r[molten.testing.first_divergence_reports] Report first-divergence diagnostics for scheduler, input, effect request/response, policy decision, action, receipt, trace, output, and state mismatches.
- [x] [serial] r[molten.testing.canonical_failure_artifacts] Emit canonical `<harness-failure-v1 ...>` Preserves artifacts for preflight, execution, replay, validation, and export failures; ensure failure artifacts carry suite/report refs and first-divergence details when available but do not satisfy pass evidence gates.
- [x] [serial] r[molten.testing.gate_receipts] Emit canonical `<gate-receipt-v1 ...>` Preserves artifacts for successful pass-evidence gate decisions, including artifact refs plus validation, replay, budget, and actor-registry check evidence.
- [ ] [serial] r[molten.testing.run_receipts] Emit Cairn receipts for suite start, step result, adapter fixture decision, expected failure, known bug, final status, and report export.
- [x] [parallel] r[molten.testing.redaction_policy] Gate report read/export with policy and apply redaction markers or encrypted refs for secrets, capabilities, and sensitive observations.

## Phase 4: Integration rails

- [x] [serial] r[molten.testing.integration_rails.transcript_step] Run executable transcript stanzas as harness steps and preserve transcript-run receipts in harness reports.
- [x] [serial] r[molten.testing.determinism_replay_core.record_replay] Integrate record and replay handler profiles so production-like runs can be captured and re-executed through the same harness rail.
- [x] [parallel] r[molten.testing.integration_rails] Add deterministic chaos mode with seeded failures, delays, drops, reorders, partitions, and resource pressure.
- [x] [parallel] r[molten.testing.harness_artifacts.identity] Memoize deterministic harness results by artifact closure, initial state, policy/schema refs, profile config, seed/log hash, runner version, and canonical suite hash.
- [x] [parallel] r[molten.testing.integration_rails] Define the operator dogfood workflow as a named harness suite with final receipt-backed report.

## Phase 5: Property and predicate testing

- [x] [serial] r[molten.testing.integration_rails.property_counterexample] Record Hegel generated inputs, shrunk counterexamples, and replay seeds as Preserves fixtures and report refs.
- [x] [serial] r[molten.testing.integration_rails.property_counterexample] Add Hegel property suites for scheduler total order, envelope canonical identity, Preserves pattern matching, trace hash stability, replay identity, and no invisible fixture mutation.
- [x] [parallel] r[molten.testing.integration_rails] Run bounded Trellis predicate checks for choreography projection, turn visibility, leases/fencing, replay guards, and resource invariants as harness checks.
- [x] [parallel] r[molten.testing.preserves_comm_rail.rendering] Add optional CI exporters such as JUnit or markdown while keeping canonical Preserves reports normative.

## Phase 6: Conformance, repro, and security rails

- [x] [serial] r[molten.testing.adapter_conformance] Add conformance suites for Iroh, Redb, Wasmtime/WASI, Steel, blob/chunk, typed storage, policy, resource, and fake network adapters using the same Preserves/effect contract.
- [x] [serial] r[molten.testing.actor_kind_interop] Add cross-kind suites for native Rust, Steel, Wasm component, adapter-backed, and remote-proxy actors communicating through the same envelope, dataspace, hostcall, policy, effect, trace, and receipt boundaries.
- [x] [serial] r[molten.testing.system_layer_suites] Add system-layer suites for demand-driven services, dependency gating, readiness/failure assertions, logical supervision, restart/shutdown, scoped service refs, auto-retraction, policy admission, and deterministic replay.
- [x] [serial] r[molten.testing.repro_bundles] Export minimal, policy-redacted repro bundles with suite/case/step refs, dependency closure, initial snapshot, policy/schema refs, profile, seed/effect log, traces, receipts, and first-divergence report.
- [x] [serial] r[molten.testing.negative_security_suites] Add negative/security suites for denied capabilities, revoked authority, malformed envelopes, noncanonical encodings, tampered content refs, invalid receipts, resource exhaustion, replay failures, and redaction leaks.
- [x] [parallel] r[molten.testing.counterexample_shrinking] Store Hegel generation seeds, shrink paths, shrunk Preserves fixtures, and replay identity as reusable regression cases.

## Phase 7: Upgrade, coverage, multi-peer, and resource rails

- [ ] [serial] r[molten.testing.upgrade_replay] Replay old traces, snapshots, schemas, policies, and artifacts against new runtime versions and require stable replay, migration receipts, or explicit compatibility diagnostics.
- [ ] [serial] r[molten.testing.boundary_coverage] Track coverage by runtime boundary: envelope routes, dataspace semantics, policy gates, effects, receipts, traces, storage, resources, replay branches, adapters, and confidentiality paths.
- [ ] [serial] r[molten.testing.deterministic_multipeer] Add deterministic multi-peer simulation for seeded or recorded peer delivery, partitions, drops, reorders, reconnects, resource limits, gossip, docs, and blob observations.
- [ ] [parallel] r[molten.testing.resource_regression] Assert deterministic budget regressions for turns, scheduler steps, mailbox depth, assertions, effects, bytes, trace volume, Wasm fuel, Steel/native checkpoints, and job-stage resources.

## Phase 8: Golden traces and flake prevention

- [ ] [serial] r[molten.testing.golden_traces] Maintain versioned golden canonical trace, receipt, and state-hash artifacts with reviewed update receipts and migration notes.
- [ ] [serial] r[molten.testing.flake_prevention] Reject flaky tests from CI/admission/release/upgrade gates unless they are deterministic, replayed, or recorded for replay; mark exploratory runs as non-replayable evidence.
