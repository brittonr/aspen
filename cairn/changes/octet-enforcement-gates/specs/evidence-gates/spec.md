## ADDED Requirements

### Requirement: Octet and Valence evidence boundary
r[molten.octet_gates.reference_boundary] The system MUST treat Octet checks and Valence function objects/fingerprints as bounded source-shape and evidence gates, not as semantic correctness proofs or replacements for runtime policy, capability checks, deterministic replay, Trellis predicates, Hegel properties, or Cairn receipt validation.

#### Scenario: Valence evidence displays caveats
r[molten.octet_gates.reference_boundary.caveats]
- GIVEN a harness report or Cairn receipt that references a Valence function object
- WHEN an operator inspects the evidence
- THEN the report identifies the function object ref, source caveats, fingerprint metadata, and the fact that it does not prove behavioral correctness

### Requirement: Critical source-surface markers
r[molten.octet_gates.source_surface_markers] The system MUST identify critical source surfaces for Octet/Valence gating, including core transitions, adapter boundaries, test capabilities, secret/capability-bearing types, harness report/oracle validators, redaction/export paths, protocol transition gates, and golden update tools.

#### Scenario: Core transition surface is classified
r[molten.octet_gates.source_surface_markers.core]
- GIVEN a function that implements a runtime core transition
- WHEN Octet evaluates the workspace
- THEN the function is classified by marker, manifest, module path, or Octet config as a core transition and receives the core-purity gate

### Requirement: Octet evidence artifacts in reports and receipts
r[molten.octet_gates.evidence_artifacts] Harness reports and Cairn receipts that rely on source gates MUST reference Octet command/config versions, findings, severity summaries, Valence function object refs, caveat summaries, fingerprints, review manifests, suppressions, and drift summaries as canonical content refs or receipt refs.

#### Scenario: CI receipt references Octet artifact bundle
r[molten.octet_gates.evidence_artifacts.ci_receipt]
- GIVEN a CI run that used Octet as an evidence gate
- WHEN the final Cairn receipt is emitted
- THEN it references the Octet artifact bundle and the harness report refs that consumed its findings

### Requirement: Core purity gate
r[molten.octet_gates.core_purity_gate] Marked core transition functions MUST be rejected or require review receipts when Octet/Valence detects ambient-effect or abort caveats, including filesystem, network, wall-clock, entropy, process, environment, database, scripting, unsafe, panic, unwrap/expect, direct adapter calls, or semantic thread-scheduling observations.

#### Scenario: Wall-clock in core transition is blocked
r[molten.octet_gates.core_purity_gate.wall_clock]
- GIVEN a function marked as a Molten core transition
- WHEN the function source uses wall-clock time directly
- THEN Octet flags the caveat and evidence gates reject the change unless a review receipt reclassifies the surface or removes the ambient observation

### Requirement: Adapter boundary evidence gate
r[molten.octet_gates.adapter_boundary_gate] Marked adapter boundary functions MUST identify their effect manifest id, handler profile compatibility, capability and policy check location, trace and receipt emission obligation, resource checkpoint behavior, replay/record behavior or non-replayable status, and structured error mapping.

#### Scenario: Adapter boundary lacks receipt obligation
r[molten.octet_gates.adapter_boundary_gate.missing_receipt]
- GIVEN an adapter function marked for a storage write effect
- WHEN Octet cannot find or validate the boundary's receipt-emission obligation
- THEN the source gate fails before the adapter can be accepted into evidence-bearing profiles

### Requirement: Effect manifest linkage
r[molten.octet_gates.effect_manifest_linkage] Adapter boundary source evidence MUST link to the Molten effect manifest entries that authorize the boundary, and evidence gates MUST fail when a marked adapter boundary has missing, stale, or mismatched effect-manifest linkage.

#### Scenario: Stale effect linkage is denied
r[molten.octet_gates.effect_manifest_linkage.stale]
- GIVEN an adapter boundary whose source marker references an effect manifest id
- WHEN that effect id no longer exists in the artifact dependency closure
- THEN Octet or the harness report validator rejects the source evidence as stale

### Requirement: Authority typing gate
r[molten.octet_gates.authority_typing_gate] Public runtime, policy, storage, harness, and adapter boundary APIs MUST NOT use raw strings, byte arrays, or generic hashes where typed actor/session/peer/run/turn ids, artifact/schema/policy/receipt/evidence/effect-log refs, capability/secret/content/snapshot/trace refs, profile markers, or staged/committed/redacted/revealed state markers are required.

#### Scenario: Stringly capability API is blocked
r[molten.octet_gates.authority_typing_gate.stringly_capability]
- GIVEN a public runtime API that accepts a raw string where a capability ref is required
- WHEN Octet evaluates boundary APIs
- THEN it flags the API and evidence gates reject it until the API parses and validates the value into a typed capability ref at the boundary

### Requirement: Harness backdoor gate
r[molten.octet_gates.harness_backdoor_gate] Harness code MUST NOT directly mutate runtime internals, stores, actor state, fixture state, policy decisions, receipts, traces, or snapshots outside explicit admitted test capabilities that emit canonical trace and receipt evidence.

#### Scenario: Direct store mutation in harness is rejected
r[molten.octet_gates.harness_backdoor_gate.store_mutation]
- GIVEN harness code that writes directly to a runtime store to set up a test
- WHEN the operation is not represented as an admitted test capability or fixture effect
- THEN Octet flags the backdoor and evidence gates reject the harness change

### Requirement: Testing-harness evidence gates
r[molten.octet_gates.testing_harness_gate] Evidence gates MUST enforce the first-class testing-harness requirements as admissibility criteria, including Preserves rail use, deterministic/replayable execution, actor-registry evidence, resource budgets, adapter conformance, security suites, repro bundles, first-divergence diagnostics, canonical failure artifacts, and canonical gate receipts. A `<harness-failure-v1 ...>` artifact MUST be accepted only as diagnostic evidence, never as a passing harness report for CI, release, admission, or upgrade gates.

#### Scenario: Failure artifact cannot pass a report gate
r[molten.octet_gates.testing_harness_gate.reject_failure]
- GIVEN a gate that requires a passing harness report for a suite, replay, validation, adapter conformance, or security check
- WHEN the available artifact is `<harness-failure-v1 ...>`
- THEN the gate preserves the failure artifact as canonical diagnostic evidence but rejects it as pass evidence

#### Scenario: Missing canonical failure artifact is a gate failure
r[molten.octet_gates.testing_harness_gate.require_failure_artifact]
- GIVEN a harness command failed during preflight, execution, replay, validation, or export and was configured with an artifact output path
- WHEN the evidence bundle contains only stderr, a nonzero exit code, or renderer-specific JSON/JUnit output
- THEN the gate rejects the bundle because no canonical Preserves failure artifact was produced

#### Scenario: Passing gate emits receipt artifact
r[molten.octet_gates.testing_harness_gate.gate_receipt]
- GIVEN a harness report or report repro bundle is accepted as passing gate evidence
- WHEN the gate decision is recorded for CI, release, admission, or upgrade use
- THEN the decision is represented by a canonical `<gate-receipt-v1 ...>` artifact containing artifact refs plus validation, replay, budget, and actor-registry check evidence

### Requirement: Production/test separation gate
r[molten.octet_gates.production_test_separation] Test-only APIs, fixture adapters, bypass capabilities, debug hooks, and exploratory non-replayable profiles MUST be feature/profile/policy isolated from production builds and MUST be denied in production profiles unless explicitly admitted for record, replay, or debug use with evidence.

#### Scenario: Test bypass leaks into production profile
r[molten.octet_gates.production_test_separation.leak]
- GIVEN a test-only bypass capability reachable from a production profile
- WHEN Octet or the harness report validator evaluates the build evidence
- THEN the evidence gate fails unless an explicit policy and receipt authorize the debug/record/replay use

### Requirement: Secret and capability rendering gate
r[molten.octet_gates.secret_rendering_gate] Secret and capability-bearing types MUST NOT expose unredacted debug, display, serialization, tracing, logging, report export, panic, or error rendering paths unless those paths route through redaction, encryption, or reveal-policy evidence.

#### Scenario: Secret ref debug output is blocked
r[molten.octet_gates.secret_rendering_gate.debug]
- GIVEN a type marked as a secret ref
- WHEN the type derives or implements debug output that renders secret material without redaction policy
- THEN Octet flags the path and evidence gates reject the change

### Requirement: Resource source-shape gate
r[molten.octet_gates.resource_shape_gate] Runtime, adapter, harness, transcript, property, and report paths MUST have deterministic bounds or checkpoints for loops, queues, recursion, deferred work, trace/report builders, Wasm fuel, Steel/native checkpoints, and materialization of content or snapshots.

#### Scenario: Unbounded report builder is blocked
r[molten.octet_gates.resource_shape_gate.report_bound]
- GIVEN a report export path that accumulates runtime traces without a declared trace-byte or record-count budget
- WHEN Octet evaluates the source surface
- THEN it flags the unbounded path and evidence gates require a budget checkpoint or review receipt

### Requirement: Fingerprint drift gate
r[molten.octet_gates.fingerprint_drift_gate] Drift in Valence function objects or Octet fingerprints for critical surfaces MUST trigger required follow-up evidence such as harness replay, Hegel property reports, golden trace updates, adapter conformance, security suites, Trellis checks, migration notes, or review receipts before CI, release, admission, or upgrade gates accept the change.

#### Scenario: Adapter fingerprint drift requires conformance
r[molten.octet_gates.fingerprint_drift_gate.adapter]
- GIVEN a changed Valence fingerprint for an adapter boundary
- WHEN CI evaluates the evidence bundle
- THEN it requires an adapter conformance report and replay or record evidence before accepting the change

### Requirement: Fail-closed Octet caveats
r[molten.octet_gates.fail_closed_caveats] Evidence gates MUST fail when required Octet or Valence artifacts, caveat summaries, function objects, fingerprints, review manifests, suppressions, or drift summaries are missing, malformed, stale, unsupported, or not linked to the relevant harness/Cairn evidence.

#### Scenario: Missing caveat summary fails evidence
r[molten.octet_gates.fail_closed_caveats.missing]
- GIVEN a harness report that claims a core transition passed source gating
- WHEN the referenced Valence caveat summary is missing or stale
- THEN the report validator rejects the claim rather than treating missing caveats as clean evidence

### Requirement: Review receipt linkage
r[molten.octet_gates.review_receipt_linkage] Octet suppressions, review manifests, caveat overrides, and fingerprint-drift approvals MUST link to Cairn receipt refs or authenticated content refs so source-gate exceptions remain auditable.

#### Scenario: Suppression lacks review receipt
r[molten.octet_gates.review_receipt_linkage.suppression]
- GIVEN an Octet suppression for a core purity caveat
- WHEN the suppression lacks a review receipt or authenticated review manifest ref
- THEN evidence gates reject the suppression for CI, release, admission, or upgrade evidence
