# Operator Workflow Specification

## Purpose

Defines the `operator-workflow` capability.

## Requirements

### Requirement: Dogfood reports are canonical pass evidence
r[molten.operator_dogfood_node_workflow.spec.report] The local dogfood workflow MUST emit a canonical report whose decision is derived from step receipts, replay status, redaction checks, and gate receipts.

#### Scenario: Complete dogfood pass
- GIVEN a clean state root and admitted operator authority
- WHEN the local dogfood workflow completes all mandatory steps
- THEN it emits a `dogfood-report-v1` with decision `pass`
- AND the report binds startup, service, remote, job, catalog, repro, gate, and shutdown receipts

#### Scenario: Missing step receipt denies
- GIVEN a dogfood workflow where a mandatory step lacks a canonical receipt
- WHEN the final report is built
- THEN the report decision is `deny`
- AND no release gate receipt is emitted

### Requirement: Release gates exclude non-replayable evidence
r[molten.operator_dogfood_node_workflow.spec.release_gate] A dogfood release gate MUST require deterministic or recorded pass evidence for mandatory steps and MUST exclude unrecorded live diagnostics.

#### Scenario: Live diagnostic does not gate release
- GIVEN a dogfood report containing a live Iroh diagnostic step without recorded delivery/effect logs
- WHEN a release gate is requested
- THEN the gate denies or excludes that step from pass evidence according to policy

### Requirement: Operator bypasses are explicit
r[molten.operator_dogfood_node_workflow.spec.no_hidden_bypass] Operator workflows MUST NOT use hidden runtime backdoors; privileged actions MUST be represented as explicit capability-bearing requests with receipts.

#### Scenario: Unauthorized operator action denied
- GIVEN an operator workflow step without required authority
- WHEN the step attempts to install or execute an artifact
- THEN Molten emits a denial receipt
- AND the dogfood report records the failed step

### Requirement: Nix dogfood local-node release check
r[molten.operator_dogfood_nix_release_check.check] Molten MUST expose a Nix check that runs `molten dogfood local-node` with an explicit temporary state root and fails closed unless the dogfood report decision passes and a release gate receipt is emitted.

#### Scenario: Nix dogfood check passes only with release gate
- GIVEN the Nix dogfood release check is built
- WHEN `molten dogfood local-node` completes in the temporary state root
- THEN the check requires a passing dogfood report and a canonical release gate receipt

r[molten.operator_dogfood_nix_release_check.nextest_dependency] The Nix dogfood release check MUST depend on the existing hermetic nextest check output so release dogfood runs only after the test suite check is available.

#### Scenario: Dogfood check is ordered after nextest
- GIVEN Nix evaluates the dogfood release check
- WHEN it realizes the check derivation
- THEN it references the nextest check output and records that dependency in the dogfood check output

r[molten.operator_dogfood_nix_release_check.artifacts] The Nix dogfood release check MUST copy the canonical dogfood report, release gate receipt, human summary, and nextest dependency marker to its output path for release review.

#### Scenario: Review artifacts are preserved
- GIVEN the dogfood release check succeeds
- WHEN an operator inspects the check output
- THEN the dogfood report, release gate receipt, summary, and after-nextest marker are present

r[molten.operator_dogfood_nix_release_check.docs] Molten SHOULD document the Nix dogfood release check and MUST state that the emitted artifacts are release evidence only, not authority, policy, provenance, resource, transport, source-gate, retention, or destructive-operation trust.

#### Scenario: Docs explain evidence-only boundary
- GIVEN an operator reads the release verification documentation
- WHEN they inspect the dogfood Nix check instructions
- THEN the docs show how to run the check and explain that the receipts do not replace subsystem gates

r[molten.operator_dogfood_nix_release_check.validation] Molten MUST validate the Nix dogfood check, Cairn gates, and relevant Rust checks before archiving this change.

#### Scenario: Change is validated before archive
- GIVEN this change is ready to archive
- WHEN validation runs
- THEN Nix dogfood check, Cairn gates, and Rust checks pass

### Requirement: Nix dogfood release evidence binds output artifacts
r[molten.operator_dogfood_nix_release_evidence.export] Molten MUST emit canonical Nix dogfood release evidence that binds the realized output path, dogfood report ref, release-gate ref, summary ref, nextest marker ref, and preserved file refs.

#### Scenario: Evidence binds output path and release gate
- GIVEN a successful Nix dogfood check output
- WHEN `molten dogfood nix-release-export` reads the output path
- THEN it emits `nix-dogfood-release-evidence-v1` with the output path ref, report ref, release-gate ref, summary ref, nextest marker ref, and file refs

### Requirement: Nix dogfood evidence can be verified
r[molten.operator_dogfood_nix_release_evidence.verify] Molten MUST provide verification receipts that recompute the Nix dogfood output refs and deny mismatches before release review trusts the evidence graph.

#### Scenario: Verification passes for matching output
- GIVEN canonical Nix dogfood evidence for an output path
- WHEN `molten dogfood nix-release-verify` recomputes the output refs
- THEN it emits `nix-dogfood-release-verify-receipt-v1` with decision `pass`
- AND the receipt binds the evidence ref, report ref, and release-gate ref

#### Scenario: Verification denies stale evidence
- GIVEN Nix dogfood evidence whose report, release-gate, summary, marker, or path refs no longer match the output path
- WHEN verification runs
- THEN it emits a deny receipt with diagnostics before release review accepts the stale refs

### Requirement: Nix check preserves evidence and verification receipts
r[molten.operator_dogfood_nix_release_evidence.nix_check] The `dogfood-local-node` Nix check MUST export and verify Nix dogfood release evidence after writing its report, release gate, summary, and nextest marker outputs.

#### Scenario: Check output contains verify receipt
- GIVEN the Nix dogfood check succeeds
- WHEN an operator inspects the check output
- THEN `nix-dogfood-evidence.preserves` and `nix-dogfood-verify.preserves` are present beside the dogfood report and release gate

### Requirement: Nix dogfood evidence is review evidence only
r[molten.operator_dogfood_nix_release_evidence.evidence_only] Nix dogfood release evidence MUST NOT grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation trust, or permission to bypass subsystem gates.

#### Scenario: Evidence does not replace subsystem gates
- GIVEN a passing Nix dogfood verification receipt
- WHEN a later subsystem performs privileged, destructive, transport, provenance-sensitive, source-gated, or retention-sensitive work
- THEN that subsystem still requires its own matching gate receipts and MUST NOT treat Nix dogfood evidence as trust authority

### Requirement: Nix dogfood evidence behavior is tested
r[molten.operator_dogfood_nix_release_evidence.tests] Molten SHOULD cover Nix dogfood release export, verification, summaries, and mismatch denial in automated tests.

#### Scenario: CLI coverage exercises export and verify
- GIVEN a local dogfood output fixture with report, release gate, summary, and nextest marker files
- WHEN tests run export and verify commands
- THEN the verification receipt passes and dogfood show can summarize both canonical artifacts

### Requirement: Operator dogfood receipts are canonical ledger artifacts
r[molten.dogfood.operator_receipt_schema] Molten MUST represent local dogfood operator receipts as canonical Preserves artifacts that bind run/workflow identity, config or policy refs, node identity refs, state hashes, child receipt refs, status, replay status, diagnostics, and redaction metadata where applicable.

#### Scenario: Final dogfood receipt binds child evidence
- GIVEN a local dogfood workflow completes
- WHEN the final dogfood report is written
- THEN the canonical report binds workflow, checkpoint, child receipt, gate, repro, status, diagnostics, and redaction-gate evidence refs

### Requirement: Dogfood receipt CLI readback
r[molten.dogfood.receipts_cli] Molten MUST expose operator receipt commands that list, show, validate, and export local dogfood receipt artifacts from the content-addressed evidence ledger.

#### Scenario: Operator validates a local dogfood receipt
- GIVEN a local dogfood run has imported operator artifacts into its ledger
- WHEN the operator runs receipt list, show, validate, and export commands for the dogfood report ref
- THEN Molten reads the canonical ledger artifact, validates the supported operator receipt schema, renders a non-normative summary, and exports canonical Preserves bytes

### Requirement: Receipt rendering is redaction-aware
r[molten.dogfood.redaction] Receipt list, show, validate, and export commands MUST avoid treating logs or unredacted text as authority and MUST render summaries as redaction-aware non-normative views over canonical Preserves receipts.

#### Scenario: Receipt export avoids log trust
- GIVEN an operator exports a local dogfood receipt
- WHEN Molten writes the exported artifact
- THEN the exported receipt remains canonical Preserves evidence
- AND rendered summaries and logs remain auxiliary views rather than primary evidence

### Requirement: Logs are auxiliary evidence only
r[molten.dogfood.no_logs_as_evidence] Molten MUST document that logs and CLI summaries are auxiliary operator aids; canonical receipts, traces, and content refs are the primary evidence for dogfood decisions.

#### Scenario: Operator inspects dogfood output
- GIVEN a dogfood run prints CLI status text
- WHEN release review evaluates the run
- THEN review uses canonical dogfood report, release gate, Nix evidence, verification receipt, trace, and content refs instead of log text

### Requirement: Local dogfood command remains the vertical slice
r[molten.dogfood.local_command] Molten MUST provide a local dogfood command that runs the deterministic local-node workflow and writes canonical report and release-gate artifacts.

#### Scenario: Local dogfood command completes
- GIVEN an empty explicit state root
- WHEN `molten dogfood local-node` runs
- THEN it writes a canonical dogfood report and release gate receipt for operator review

### Requirement: Local dogfood exercises runtime boundaries
r[molten.dogfood.vertical_slice] The local dogfood workflow MUST exercise config or policy refs, node identity, artifact installation, handler or service binding, local dataspace exchange, receipt storage, transcript or repro execution, and cleanup or retention review evidence.

#### Scenario: Dogfood report covers the vertical slice
- GIVEN the local dogfood workflow succeeds
- WHEN the report is parsed
- THEN it includes mandatory step evidence for startup, service or handler execution, remote-shaped delivery, job execution, catalog/readback, repro verification, retention review, and shutdown

### Requirement: Local dogfood state can be preserved for inspection
r[molten.dogfood.leave_running] Molten SHOULD allow local dogfood state and ledger artifacts to remain available for operator inspection after the workflow completes.

#### Scenario: Operator inspects preserved state
- GIVEN a local dogfood workflow ran with an explicit state root
- WHEN the command exits
- THEN the state root ledger remains available for receipt list, show, validate, and export commands

### Requirement: Dogfood final receipt summarizes outcome
r[molten.dogfood.final_receipt] Molten MUST store a final dogfood report receipt that records success or failure with child receipt refs, workflow refs, checkpoint refs, final state refs, final status, and diagnostics.

#### Scenario: Final report denies incomplete evidence
- GIVEN a mandatory dogfood step lacks canonical receipt evidence
- WHEN the final dogfood report is built
- THEN the report decision is `deny` and diagnostics name the missing evidence

### Requirement: Dogfood replay status is validated
r[molten.dogfood.replay_validation] Dogfood reports MUST require deterministic or recorded replay status for mandatory release evidence and MUST include first-divergence diagnostics when replay-bound verification fails.

#### Scenario: Non-replayable mandatory step denies release evidence
- GIVEN a mandatory dogfood step is marked non-replayable
- WHEN the report is evaluated
- THEN the dogfood report denies release evidence before a release gate is accepted

### Requirement: Cluster-backed receipt readback is planned but not required locally
r[molten.dogfood.cluster_readback_plan] Molten MAY add cluster-backed receipt readback later, but the local dogfood receipt CLI MUST work without production cluster storage.

#### Scenario: Local readback works before cluster storage
- GIVEN only a local dogfood evidence ledger exists
- WHEN receipt readback commands run
- THEN they operate on the local content-addressed ledger without requiring Raft or cluster services

### Requirement: Dogfood receipt CLI is tested
r[molten.dogfood.cli_tests] Molten SHOULD test the local dogfood receipt list, show, validate, export, Nix evidence export, and Nix evidence verification CLI paths.

#### Scenario: CLI test covers receipt readback
- GIVEN a CLI test runs local dogfood
- WHEN it lists, shows, validates, exports, and verifies dogfood receipts
- THEN the commands pass for current evidence and emit deny verification receipts for stale Nix refs

### Requirement: Dogfood receipt graph integrity is tested
r[molten.dogfood.property_tests] Molten SHOULD test dogfood receipt child graph integrity and redacted export stability with deterministic examples or property tests.

#### Scenario: Receipt graph remains stable
- GIVEN a dogfood report with child receipt refs and redaction-safe export
- WHEN tests recompute canonical refs and export the report
- THEN the exported artifact ref matches the original report ref and child receipt refs remain stable

### Requirement: Release evidence bundle export
r[molten.operator_dogfood_release_evidence_bundle.export] Molten MUST export a canonical release evidence bundle that binds the realized dogfood Nix output path, dogfood report ref, release gate ref, Nix dogfood evidence ref, Nix verify receipt ref, summary ref, nextest marker ref, nextest check path, and preserved member file refs.

#### Scenario: Bundle binds dogfood release members
- GIVEN a successful dogfood Nix output containing dogfood report, release gate, summary, nextest marker, Nix evidence, and Nix verify receipt files
- WHEN `molten dogfood release-bundle-export` reads the output path
- THEN it emits `release-evidence-bundle-v1` with all member refs and review checks bound canonically

### Requirement: Release evidence bundle verification
r[molten.operator_dogfood_release_evidence_bundle.verify] Molten MUST verify release evidence bundles by recomputing output refs and MUST emit a canonical deny receipt for stale, missing, or tampered bundle members.

#### Scenario: Bundle verification passes for matching output
- GIVEN a canonical release evidence bundle for a dogfood Nix output
- WHEN `molten dogfood release-bundle-verify` recomputes the output refs
- THEN it emits `release-evidence-bundle-verify-receipt-v1` with decision `pass`
- AND the receipt binds the bundle ref, dogfood report ref, release gate ref, Nix evidence ref, and Nix verify receipt ref

#### Scenario: Bundle verification denies stale output
- GIVEN a release evidence bundle whose report, release gate, Nix evidence, Nix verify, summary, nextest marker, or output path refs no longer match the output path
- WHEN verification runs
- THEN it emits `release-evidence-bundle-verify-receipt-v1` with decision `deny`
- AND diagnostics identify the stale or missing member before release review accepts the graph

### Requirement: Nix check preserves release bundles
r[molten.operator_dogfood_release_evidence_bundle.nix_check] The `dogfood-local-node` Nix check MUST export and verify a release evidence bundle after Nix dogfood evidence verification succeeds, and MUST preserve the bundle and bundle verify receipt in the check output.

#### Scenario: Check output contains bundle evidence
- GIVEN the Nix dogfood check succeeds
- WHEN an operator inspects the check output
- THEN `release-evidence-bundle.preserves` and `release-evidence-bundle-verify.preserves` are present beside the dogfood report, release gate, Nix evidence, and Nix verify receipt

### Requirement: Release bundles are evidence only
r[molten.operator_dogfood_release_evidence_bundle.evidence_only] Release evidence bundles MUST NOT grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation trust, or permission to bypass subsystem gates.

#### Scenario: Bundle does not replace subsystem gates
- GIVEN a passing release bundle verification receipt
- WHEN a later subsystem performs privileged, destructive, transport, provenance-sensitive, source-gated, or retention-sensitive work
- THEN that subsystem still requires its own matching gate receipts and MUST NOT treat the release bundle as trust authority

### Requirement: Release bundle behavior is tested and documented
r[molten.operator_dogfood_release_evidence_bundle.tests] Molten SHOULD cover release bundle export, verification pass, stale-member denial, summaries, Nix preservation, and operator documentation in automated tests and docs.

#### Scenario: CLI coverage exercises bundle verification
- GIVEN a local dogfood output fixture with report, release gate, Nix evidence, Nix verify receipt, summary, and nextest marker files
- WHEN tests run bundle export and verify commands
- THEN the bundle verification receipt passes for current refs and denies stale marker refs with diagnostics

### Requirement: Release bundle documentation
r[molten.operator_dogfood_release_evidence_bundle.docs] Molten SHOULD document the release evidence bundle commands and MUST state that bundle artifacts are review evidence only, not authority or subsystem trust.

#### Scenario: Docs explain bundle outputs
- GIVEN an operator reads the release verification documentation
- WHEN they inspect the dogfood release bundle instructions
- THEN the docs show how to run the bundle export and verify commands and explain the evidence-only boundary

### Requirement: Production release-candidate validation matrix
r[molten.prod_release_candidate.full_validation_matrix] Molten MUST define a production release-candidate validation matrix that binds the current Rust validation, hermetic nextest, Nix checks, Cairn strict validation, Octet strict source gate, dogfood-local-node check, release bundle verification, promotion summary, and export verification evidence for the same candidate.

#### Scenario: Candidate passes only with current full evidence
- GIVEN a candidate source tree and Nix input set
- WHEN the production release-candidate gate evaluates the candidate
- THEN it accepts only passing, current, mutually bound validation evidence for that candidate
- AND it emits deny diagnostics for missing, stale, failed, or mismatched evidence.

### Requirement: Current source-gate evidence is required
r[molten.prod_release_candidate.source_gate_current] Molten MUST require current Octet source-gate evidence for production release candidates and MUST distinguish source-remediated-zero from configuration-clean evidence that still depends on disabled lint-family caveats.

#### Scenario: Configuration-clean caveat limits promotion
- GIVEN a candidate whose strict Octet gate passes only under documented disabled lint-family caveats
- WHEN the production release-candidate gate is asked to approve broad production use
- THEN the gate denies broad promotion or records the caveat as a pilot-scope limiter rather than claiming source-remediated zero.

### Requirement: Release-candidate receipt binds promotion evidence
r[molten.prod_release_candidate.evidence_bundle_promotion] Molten MUST emit a canonical production release-candidate receipt that binds dogfood output refs, release evidence bundle verification refs, promotion gate refs, signed promotion or keyring verification refs where available, promotion summary refs, export verification refs, and source-gate refs.

#### Scenario: Stale release bundle denies candidate
- GIVEN a release evidence bundle whose verified members do not match the candidate dogfood output or source-gate refs
- WHEN the production release-candidate receipt is generated
- THEN it emits a deny decision with diagnostics before any production pilot decision can pass.

### Requirement: Production pilot decision is explicit and scoped
r[molten.prod_release_candidate.pilot_decision] Molten MUST record production-pilot decisions explicitly, including allowed workloads, denied workloads, rollback triggers, stop-the-line conditions, operator review refs, and evidence-only caveats.

#### Scenario: Candidate is accepted for limited pilot only
- GIVEN all required release-candidate evidence passes but known caveats remain for live distributed soak or source-remediated-zero completeness
- WHEN the pilot decision is recorded
- THEN the receipt may pass only for the named constrained pilot scope
- AND it MUST deny or exclude broad customer-critical or irreversible destructive workloads.
