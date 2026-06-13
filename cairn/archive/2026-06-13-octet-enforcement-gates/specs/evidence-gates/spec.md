## ADDED Requirements

### Requirement: Octet and Valence evidence boundary
r[molten.octet_gates.reference_boundary] Molten MUST treat Octet checks and Valence function objects/fingerprints as bounded source-shape and evidence gates, not as semantic correctness proofs or replacements for runtime policy, capability checks, deterministic replay, Trellis predicates, Hegel properties, or Cairn receipt validation.

#### Scenario: Valence evidence displays caveats
- GIVEN a harness report or Cairn receipt that references a Valence function object
- WHEN an operator inspects the evidence
- THEN the report identifies the function object ref, source caveats, fingerprint metadata, and the fact that it does not prove behavioral correctness.

### Requirement: Critical source-surface markers
r[molten.octet_gates.source_surface_markers] Molten MUST identify critical source surfaces for Octet/Valence gating by marker attributes, module paths, object-corpus source paths, remediation-plan critical-surface inventory, or Octet config. Initial surfaces include core transitions, adapter boundaries, test capabilities, secret/capability-bearing types, harness report/oracle validators, redaction/export paths, protocol transition gates, and golden update tools.

#### Scenario: Core transition surface is classified
- GIVEN a function or module implements a runtime core transition
- WHEN Octet evidence is generated
- THEN the surface is classified by marker, manifest, module path, object corpus, remediation inventory, or Octet config
- AND source-gate evidence records the applicable critical surface.

### Requirement: Octet evidence artifacts in reports and receipts
r[molten.octet_gates.evidence_artifacts] Harness reports and Cairn receipts that rely on source gates MUST reference Octet command/config versions, findings, severity summaries, structured finding indexes, Valence function object refs or object-corpus refs, caveat summaries, fingerprints, review manifests, suppressions, and drift summaries as canonical content refs or receipt refs.

#### Scenario: CI receipt references Octet artifact bundle
- GIVEN a CI run that used Octet as an evidence gate
- WHEN the final Cairn or Molten receipt is emitted
- THEN it references the Octet artifact bundle and the harness report refs that consumed its findings.

### Requirement: Octet CI command shape is explicit
r[molten.octet_gates.ci_command_shape] Molten MUST document and support an explicit source-gate command shape that includes `cargo octet check`, focused object-corpus/fingerprint evidence, artifact import, strict gate receipt generation, remediation-plan evidence, harness tests, and Cairn strict validation.

#### Scenario: Strict source gate command emits canonical receipt
- GIVEN the documented strict Octet source-gate sequence is run
- WHEN artifact import and gate commands complete
- THEN Molten emits canonical artifact-ledger, fingerprint, and `octet-gate-receipt-v1` evidence refs.

### Requirement: Core purity gate
r[molten.octet_gates.core_purity_gate] Marked core transition functions MUST be rejected or require review receipts when Octet/Valence detects ambient-effect or abort caveats, including filesystem, network, wall-clock, entropy, process, environment, database, scripting, unsafe, panic, unwrap/expect, direct adapter calls, or semantic thread-scheduling observations.

#### Scenario: Wall-clock in core transition is blocked
- GIVEN a function marked as a Molten core transition
- WHEN the function source uses wall-clock time directly
- THEN Octet flags the caveat
- AND evidence gates reject the change unless review evidence reclassifies or removes the ambient observation.

### Requirement: Adapter boundary evidence gate
r[molten.octet_gates.adapter_boundary_gate] Marked adapter boundary functions MUST identify their effect manifest id, handler profile compatibility, capability and policy check location, trace and receipt emission obligation, resource checkpoint behavior, replay/record behavior or non-replayable status, and structured error mapping.

#### Scenario: Adapter boundary lacks receipt obligation
- GIVEN an adapter function marked for a storage write effect
- WHEN Octet cannot find or validate the boundary's receipt-emission obligation
- THEN the source gate fails before the adapter can be accepted into evidence-bearing profiles.

### Requirement: Effect manifest linkage
r[molten.octet_gates.effect_manifest_linkage] Adapter boundary source evidence MUST link to the Molten effect manifest entries that authorize the boundary, and evidence gates MUST fail when a marked adapter boundary has missing, stale, or mismatched effect-manifest linkage.

#### Scenario: Stale effect linkage is denied
- GIVEN an adapter boundary whose source marker references an effect manifest id
- WHEN that effect id no longer exists in the artifact dependency closure
- THEN Octet or the harness report validator rejects the source evidence as stale.

### Requirement: Adapter conformance runs on boundary drift
r[molten.octet_gates.adapter_conformance_trigger] Drift in adapter boundary source fingerprints SHOULD trigger adapter conformance, replay, or golden evidence before release or admission gates accept the changed boundary.

#### Scenario: Adapter fingerprint drift requires conformance
- GIVEN a changed Valence or object-corpus fingerprint for an adapter boundary
- WHEN CI evaluates the evidence bundle
- THEN it requires adapter conformance or replay evidence before accepting the change.

### Requirement: Authority typing gate
r[molten.octet_gates.authority_typing_gate] Public runtime, policy, storage, harness, and adapter boundary APIs MUST NOT use raw strings, byte arrays, or generic hashes where typed actor/session/peer/run/turn ids, artifact/schema/policy/receipt/evidence/effect-log refs, capability/secret/content/snapshot/trace refs, profile markers, or staged/committed/redacted/revealed state markers are required.

#### Scenario: Stringly capability API is blocked
- GIVEN a public runtime API that accepts a raw string where a capability ref is required
- WHEN Octet evaluates boundary APIs
- THEN it flags the API
- AND evidence gates reject it until the API parses and validates the value into a typed capability ref at the boundary.

### Requirement: Harness backdoor gate
r[molten.octet_gates.harness_backdoor_gate] Harness code MUST NOT directly mutate runtime internals, stores, actor state, fixture state, policy decisions, receipts, traces, or snapshots outside explicit admitted test capabilities that emit canonical trace and receipt evidence.

#### Scenario: Direct store mutation in harness is rejected
- GIVEN harness code writes directly to a runtime store to set up a test
- WHEN the operation is not represented as an admitted test capability or fixture effect
- THEN Octet flags the backdoor
- AND evidence gates reject the harness change.

### Requirement: Testing-harness evidence gates
r[molten.octet_gates.testing_harness_gate] Evidence gates MUST enforce the first-class testing-harness requirements as admissibility criteria, including Preserves rail use, deterministic/replayable execution, actor-registry evidence, resource budgets, adapter conformance, security suites, repro bundles, first-divergence diagnostics, canonical failure artifacts, and canonical gate receipts. A `<harness-failure-v1 ...>` artifact MUST be accepted only as diagnostic evidence, never as a passing harness report for CI, release, admission, or upgrade gates.

#### Scenario: Failure artifact cannot pass a report gate
- GIVEN a gate that requires a passing harness report for a suite, replay, validation, adapter conformance, or security check
- WHEN the available artifact is `<harness-failure-v1 ...>`
- THEN the gate preserves the failure artifact as canonical diagnostic evidence
- AND rejects it as pass evidence.

#### Scenario: Missing canonical failure artifact is a gate failure
- GIVEN a harness command failed during preflight, execution, replay, validation, or export and was configured with an artifact output path
- WHEN the evidence bundle contains only stderr, a nonzero exit code, or renderer-specific JSON/JUnit output
- THEN the gate rejects the bundle because no canonical Preserves failure artifact was produced.

#### Scenario: Passing gate emits receipt artifact
- GIVEN a harness report or report repro bundle is accepted as passing gate evidence
- WHEN the gate decision is recorded for CI, release, admission, or upgrade use
- THEN the decision is represented by a canonical `<gate-receipt-v1 ...>` artifact containing artifact refs plus validation, replay, budget, and actor-registry check evidence.

### Requirement: Production/test separation gate
r[molten.octet_gates.production_test_separation] Test-only APIs, fixture adapters, bypass capabilities, debug hooks, and exploratory non-replayable profiles MUST be feature/profile/policy isolated from production builds and MUST be denied in production profiles unless explicitly admitted for record, replay, or debug use with evidence.

#### Scenario: Test bypass leaks into production profile
- GIVEN a test-only bypass capability reachable from a production profile
- WHEN Octet or the harness report validator evaluates the build evidence
- THEN the evidence gate fails unless an explicit policy and receipt authorize the debug, record, or replay use.

### Requirement: Secret and capability rendering gate
r[molten.octet_gates.secret_rendering_gate] Secret and capability-bearing types MUST NOT expose unredacted debug, display, serialization, tracing, logging, report export, panic, or error rendering paths unless those paths route through redaction, encryption, or reveal-policy evidence.

#### Scenario: Secret ref debug output is blocked
- GIVEN a type marked as a secret ref
- WHEN the type derives or implements debug output that renders secret material without redaction policy
- THEN Octet flags the path
- AND evidence gates reject the change.

### Requirement: Resource source-shape gate
r[molten.octet_gates.resource_shape_gate] Runtime, adapter, harness, transcript, property, and report paths MUST have deterministic bounds or checkpoints for loops, queues, recursion, deferred work, trace/report builders, Wasm fuel, Steel/native checkpoints, and materialization of content or snapshots.

#### Scenario: Unbounded report builder is blocked
- GIVEN a report export path accumulates runtime traces without a declared trace-byte or record-count budget
- WHEN Octet evaluates the source surface
- THEN it flags the unbounded path
- AND evidence gates require a budget checkpoint or review receipt.

### Requirement: Fingerprint drift gate
r[molten.octet_gates.fingerprint_drift_gate] Drift in Valence function objects, object corpus, or Octet fingerprints for critical surfaces MUST trigger required follow-up evidence such as harness replay, Hegel property reports, golden trace updates, adapter conformance, security suites, Trellis checks, migration notes, or review receipts before CI, release, admission, or upgrade gates accept the change.

#### Scenario: Adapter fingerprint drift requires conformance
- GIVEN a changed Valence fingerprint for an adapter boundary
- WHEN CI evaluates the evidence bundle
- THEN it requires an adapter conformance report and replay or record evidence before accepting the change.

### Requirement: Fail-closed Octet caveats
r[molten.octet_gates.fail_closed_caveats] Evidence gates MUST fail when required Octet or Valence artifacts, caveat summaries, function objects, object-corpus refs, fingerprints, review manifests, suppressions, or drift summaries are missing, malformed, stale, unsupported, or not linked to the relevant harness/Cairn evidence.

#### Scenario: Missing caveat summary fails evidence
- GIVEN a harness report claims a core transition passed source gating
- WHEN the referenced Valence caveat summary is missing or stale
- THEN the report validator rejects the claim rather than treating missing caveats as clean evidence.

### Requirement: Review receipt linkage
r[molten.octet_gates.review_receipt_linkage] Octet suppressions, review manifests, caveat overrides, and fingerprint-drift approvals MUST link to Cairn receipt refs or authenticated content refs so source-gate exceptions remain auditable.

#### Scenario: Suppression lacks review receipt
- GIVEN an Octet suppression for a core purity caveat
- WHEN the suppression lacks a review receipt or authenticated review manifest ref
- THEN evidence gates reject the suppression for CI, release, admission, or upgrade evidence.

### Requirement: Core purity source-gate tests
r[molten.octet_gates.core_purity_tests] Molten SHOULD test that strict source gates deny ambient-effect, abort, stale metadata, malformed artifact, missing object-corpus, and missing fingerprint cases before downstream consumers accept the source evidence.

#### Scenario: Warning-only status denies strict gate
- GIVEN Octet artifacts with warning-only status
- WHEN the strict source-gate evaluator runs
- THEN it emits a deny receipt with diagnostics rather than pass evidence.

### Requirement: Authority typing source-gate tests
r[molten.octet_gates.authority_typing_tests] Molten SHOULD test that stringly capability, receipt, schema, content-ref, or source-gate mixups are rejected by source-gate validation or downstream consumers before runtime admission.

#### Scenario: Raw source summary is not a typed gate receipt
- GIVEN a downstream consumer receives only an Octet summary ref
- WHEN source-gate validation runs
- THEN validation denies because a canonical typed `octet-gate-receipt-v1` value is required.

### Requirement: Harness backdoor source-gate tests
r[molten.octet_gates.harness_backdoor_tests] Molten SHOULD test that invisible harness store mutation, private runtime backdoors, and canonical failure artifacts cannot be accepted as passing gate evidence.

#### Scenario: Failure artifact is diagnostic only
- GIVEN a canonical harness failure artifact
- WHEN a pass gate requires a harness report
- THEN the failure artifact remains diagnostic evidence only and cannot satisfy the pass gate.

### Requirement: Adapter boundary source-gate tests
r[molten.octet_gates.adapter_boundary_tests] Molten SHOULD test that adapter boundaries missing effect, trace, receipt, resource, replay, or fingerprint evidence are denied by source-gate or source-gate-validation logic.

#### Scenario: Missing fingerprint evidence denies adapter gate
- GIVEN an otherwise pass-shaped Octet gate receipt without object-corpus fingerprint evidence
- WHEN source-gate validation runs
- THEN the validation denies with missing fingerprint coverage diagnostics.

### Requirement: Fingerprint drift source-gate tests
r[molten.octet_gates.fingerprint_drift_tests] Molten SHOULD test that stale config/profile hashes, changed structured findings, missing object-corpus refs, warning-baseline regressions, and unreviewed critical findings deny or require review evidence.

#### Scenario: Stale source gate denies
- GIVEN a previously passing Octet gate receipt with stale config or profile hash metadata
- WHEN source-gate validation runs
- THEN the validation denies and records deterministic stale-evidence diagnostics.
