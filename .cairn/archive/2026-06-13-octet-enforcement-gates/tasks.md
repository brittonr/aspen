## Phase 1: Octet evidence model

- [x] [serial] r[molten.octet_gates.reference_boundary] Define Octet/Valence as bounded source/evidence gates and document that they do not prove semantic correctness or replace runtime policy/replay.
- [x] [serial] r[molten.octet_gates.source_surface_markers] Define initial marker/config model for core transitions, adapter boundaries, test capabilities, secret/capability-bearing types, and critical golden/report validators.
- [x] [serial] r[molten.octet_gates.evidence_artifacts] Define how Octet findings, SARIF/sidecars, Valence function objects, caveat summaries, fingerprints, review manifests, and suppressions become content refs in harness reports and Cairn receipts.
- [x] [parallel] r[molten.octet_gates.ci_command_shape] Add the initial CI command shape for `cargo octet check`, fingerprint checks, harness suites, and Cairn validation.

## Phase 2: Core and adapter gates

- [x] [serial] r[molten.octet_gates.core_purity_gate] Enforce or review ambient-effect caveats for marked core transitions: fs, net, wall-clock, entropy, process, environment, database, scripting, unsafe, panic, unwrap/expect, direct adapter calls, and semantic thread-scheduling observations.
- [x] [serial] r[molten.octet_gates.adapter_boundary_gate] Require marked adapter boundaries to identify effect manifest, handler profile compatibility, capability/policy checks, trace/receipt emission, resource checkpoints, replay/record behavior, and structured error mapping.
- [x] [serial] r[molten.octet_gates.effect_manifest_linkage] Link adapter boundary source surfaces to Molten effect manifest ids and fail evidence gates when linkage is missing or stale.
- [x] [parallel] r[molten.octet_gates.adapter_conformance_trigger] Trigger adapter conformance and replay/golden suites when adapter boundary fingerprints drift.

## Phase 3: Authority, harness, and production separation

- [x] [serial] r[molten.octet_gates.authority_typing_gate] Flag public boundary APIs that use raw strings, bytes, or generic hashes where typed ids, refs, capabilities, secrets, receipts, effect logs, profiles, or state markers are required.
- [x] [serial] r[molten.octet_gates.harness_backdoor_gate] Flag harness code that mutates runtime internals, fixture state, stores, receipts, actor state, or policy outcomes outside explicit admitted test capabilities with trace/receipt evidence.
- [x] [serial] r[molten.octet_gates.testing_harness_gate] Enforce first-class testing-harness requirements as evidence gates, including rejection of `<harness-failure-v1 ...>` as pass evidence, canonical `<gate-receipt-v1 ...>` success artifacts, and rejection of stderr/exit-status-only failures when a canonical artifact output was requested.
- [x] [serial] r[molten.octet_gates.production_test_separation] Enforce feature/profile separation so test-only APIs, fixture adapters, bypass capabilities, debug hooks, and exploratory profiles are unreachable from production profiles unless explicitly admitted and evidenced.
- [x] [parallel] r[molten.octet_gates.secret_rendering_gate] Flag secret/capability-bearing types that expose unredacted debug, display, serialization, tracing, logging, report export, or panic/error rendering paths.

## Phase 4: Resource and fingerprint gates

- [x] [serial] r[molten.octet_gates.resource_shape_gate] Flag unbounded loops, queues, deferred work, recursion, trace/report builders, and actor/adapter execution paths without deterministic budget/yield/cancel checkpoints.
- [x] [serial] r[molten.octet_gates.fingerprint_drift_gate] Require harness replay, property/golden reports, adapter conformance, security suites, Trellis checks, migration notes, or review receipts when Valence fingerprints drift on critical surfaces.
- [x] [serial] r[molten.octet_gates.fail_closed_caveats] Fail evidence gates when required Octet/Valence artifacts, caveat summaries, function objects, review manifests, or suppressions are missing, malformed, stale, or unsupported.
- [x] [parallel] r[molten.octet_gates.review_receipt_linkage] Link review manifests and suppressions to Cairn receipt refs or content refs so review evidence is auditable.

## Phase 5: Tests and dogfood

- [x] [serial] r[molten.octet_gates.core_purity_tests] Add focused fixtures showing core purity violations are blocked or require review receipts.
- [x] [serial] r[molten.octet_gates.authority_typing_tests] Add fixtures showing stringly capability/receipt/schema/content ref mixups are flagged before runtime admission.
- [x] [serial] r[molten.octet_gates.harness_backdoor_tests] Add fixtures showing invisible harness store mutation or private runtime backdoors are rejected.
- [x] [parallel] r[molten.octet_gates.adapter_boundary_tests] Add fixtures showing adapter boundaries without effect/trace/receipt/resource/replay evidence are rejected.
- [x] [parallel] r[molten.octet_gates.fingerprint_drift_tests] Add fixtures showing fingerprint drift requires the expected follow-up harness or review evidence.
