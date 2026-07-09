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

### Requirement: Requirement-centered proof readback
r[molten.operator.proof_readback.requirement_view] Molten SHOULD provide a deterministic proof readback grouped by requirement id for local and release review.

#### Scenario: Readback names requirement evidence
- GIVEN a traceability manifest with covered requirements
- WHEN proof readback is rendered
- THEN each requirement section names its positive and negative evidence refs.

### Requirement: Readback shows evidence chain
r[molten.operator.proof_readback.evidence_chain] Proof readbacks SHOULD show verification-run receipts, aggregate proof manifests, child obligation refs, artifact refs, and gate receipts that explain how coverage was satisfied.

#### Scenario: Aggregate proof expands to children
- GIVEN a requirement covered by an aggregate proof manifest
- WHEN readback renders the requirement
- THEN it lists the child obligation refs that satisfy the requirement.

### Requirement: Readback includes scope caveats
r[molten.operator.proof_readback.scope_caveats] Proof readbacks MUST include explicit caveats that summaries are non-normative and do not grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation trust, or permission to bypass subsystem gates.

#### Scenario: Readback cannot override deny receipt
- GIVEN a canonical gate receipt with decision `deny`
- WHEN a readback is rendered
- THEN the readback cannot present that evidence as pass and must identify the deny decision.

### Requirement: Readback renders actionable gaps
r[molten.operator.proof_readback.gap_diagnostics] Proof readbacks SHOULD group missing-positive, missing-negative, stale-reference, unsupported, and exempt entries with actionable next evidence requirements.

#### Scenario: Missing negative is visible
- GIVEN a requirement missing negative coverage
- WHEN readback is rendered
- THEN the requirement appears in a missing-negative group with the required evidence kind.

### Requirement: Proof readback CLI
r[molten.operator.proof_readback.cli] Molten SHOULD expose a CLI command or release-review surface that renders proof readbacks from traceability manifests and proof receipts.

#### Scenario: Operator renders release proof readback
- GIVEN a release traceability manifest and proof receipt set
- WHEN the operator invokes the readback command
- THEN Molten renders a compact deterministic summary and can write a canonical readback artifact.

### Requirement: Readback Hegel properties
r[molten.operator.proof_readback.hegel_properties] Proof readback rendering SHOULD include Hegel RS property tests for stable ordering, duplicate suppression, gap grouping, summary-count consistency, and non-normative caveat preservation.

#### Scenario: Generated readback remains sorted
- GIVEN Hegel RS generates an unordered set of requirement entries
- WHEN readback rendering runs
- THEN the rendered requirement groups are deterministic and summary counts match the canonical entries.

### Requirement: Proof readback documentation
r[molten.operator.proof_readback.docs] Operator documentation SHOULD explain how to inspect proof readbacks, follow evidence refs, identify gaps, and treat readbacks as rendered views over canonical receipts.

#### Scenario: Reviewer follows readback docs
- GIVEN a reviewer receives a release proof readback
- WHEN they follow the documentation
- THEN they can locate canonical receipts for positive, negative, stale, and exempt evidence.

### Requirement: Release evidence workflow is replay ordered
r[molten.release_workflow_state_proof.ordered_workflow] Molten MUST prove that release evidence proceeds through dogfood evidence, bundle export, bundle verify, signed-member verification, release promotion, signed promotion verification, readback summary, archive export, and archive verification before a release review can pass.

#### Scenario: Promotion before bundle verify denies
- GIVEN a release promotion request without a current passing bundle verification receipt
- WHEN release promotion is evaluated
- THEN promotion decision is `deny`
- AND diagnostics identify missing or stale bundle verification.

### Requirement: Release signatures bind member purpose and key state
r[molten.release_workflow_state_proof.signature_binding] Molten MUST prove that signed release members and signed promotion receipts bind the expected subject ref, signer key id, purpose, key currentness, and revocation state.

#### Scenario: Wrong-purpose signature denies bundle verification
- GIVEN a signed member with purpose `release-promotion`
- WHEN bundle verification requires purpose `release-evidence`
- THEN verification decision is `deny`
- AND diagnostics identify wrong signature purpose.

### Requirement: Release evidence remains evidence-only
r[molten.release_workflow_state_proof.evidence_only_boundary] Molten MUST prove that release bundles, promotion receipts, signed receipts, summaries, replay indexes, and export verification receipts do not grant authority, policy, provenance, source-gate, retention, transport, resource, or destructive-operation trust.

#### Scenario: Release evidence cannot bypass subsystem gate
- GIVEN a passing release promotion receipt
- WHEN a destructive or privileged subsystem attempts to use it instead of its normal gate evidence
- THEN the subsystem gate decision is `deny`
- AND diagnostics identify the missing subsystem-specific evidence.

### Requirement: Release evidence refresh binds one current candidate
r[molten.release_evidence_refresh.current_candidate_matrix] Release evidence refresh SHOULD bind Rust validation, hermetic nextest, dogfood local-node, replay evidence, release bundle verification, promotion, summary, export manifest, and export verification to the same candidate source tree and Nix input set.

#### Scenario: Candidate evidence is mutually current
- GIVEN a candidate tree after proof-affecting changes land
- WHEN release evidence refresh runs
- THEN every release-review artifact references the same candidate evidence graph
- AND stale evidence from a previous candidate is not reported as current.

### Requirement: Dogfood readback names refreshed evidence
r[molten.release_evidence_refresh.dogfood_readback] Dogfood release readback SHOULD name the refreshed dogfood report, release gate, replay verify, replay index, Nix evidence, Nix verify, bundle verify, promotion, summary, export manifest, and export verify refs needed for review.

#### Scenario: Readback follows dogfood output refs
- GIVEN a dogfood-local-node check output for the candidate
- WHEN an operator reads the release summary or README evidence notes
- THEN the listed refs and output paths identify the current dogfood evidence rather than a stale prior run.

### Requirement: Release bundle graph readback is verified
r[molten.release_evidence_refresh.bundle_graph] Release evidence refresh SHOULD verify release bundle members, signed members, promotion, signed promotion where available, summary, export manifest, archive, and export verification against one candidate evidence graph.

#### Scenario: Bundle graph readback follows one candidate
- GIVEN a refreshed dogfood local-node output for the candidate
- WHEN release bundle verification, promotion, summary, export, and export verification run
- THEN their refs identify artifacts from the same candidate evidence graph
- AND stale or mismatched members are not reported as current.

### Requirement: Refreshed release graph denies stale members
r[molten.release_evidence_refresh.stale_denial] Release evidence refresh MUST preserve deny behavior for missing, duplicate, stale, unsigned, wrong-purpose, wrong-signer, revoked, or tampered release members.

#### Scenario: Stale member cannot refresh evidence
- GIVEN a release bundle member from an older candidate
- WHEN bundle verification or release readback evaluates the refreshed graph
- THEN verification denies with stale or mismatched member diagnostics.

### Requirement: Release evidence notes are evidence-only
r[molten.release_evidence_refresh.docs] Release evidence documentation MUST state that refreshed readback artifacts remain review evidence only and do not grant authority, policy, provenance, source-gate, resource, transport, retention, destructive-operation, or deployment trust.

#### Scenario: Documentation preserves caveats
- GIVEN refreshed release evidence paths are added to operator notes
- WHEN a reviewer follows the notes
- THEN the notes identify canonical receipts and retain the evidence-only caveat for every rendered summary.

### Requirement: External live pilot scope is explicit
r[molten.external_live_pilot_soak.scope_model] Molten MUST model external live pilot scope with named hosts or nodes, allowed workloads, denied workloads, rollback triggers, stop-the-line conditions, operator review refs, and evidence-only caveats.

#### Scenario: Over-broad pilot scope denies
- GIVEN pilot evidence that only covers a constrained internal workload
- WHEN the pilot decision requests broad production or irreversible destructive workload approval
- THEN the pilot decision denies or excludes the over-broad scope
- AND diagnostics identify the missing evidence classes.

### Requirement: External pilot operator runbook is reproducible
r[molten.external_live_pilot_soak.operator_runbook] Molten SHOULD document operator-runbook steps for multi-host setup, state roots, live tickets, authority grants, node-control workflow, artifact collection, replay readback, rollback, and teardown.

#### Scenario: Operator can rerun pilot collection
- GIVEN an operator follows the external pilot runbook
- WHEN the pilot workflow completes or denies
- THEN the runbook identifies the canonical artifacts to collect for review
- AND diagnostic logs remain secondary to receipts.

### Requirement: External pilot evidence bundle binds child workflows
r[molten.external_live_pilot_soak.evidence_bundle] External pilot evidence MUST bind child refs for node-control live workflow, peer admission, authority grant, remote dataspace or service exchange, blob-ref job execution, coordination apply, replay verification, network diagnostics, resource envelope, and rollback or stop-the-line evidence.

#### Scenario: Complete pilot bundle passes scope checks
- GIVEN all required child workflow receipts pass for the named pilot scope
- WHEN the pilot evidence bundle is validated
- THEN the bundle decision may pass for that constrained scope
- AND the bundle remains review evidence only.

### Requirement: External pilot positive workflow is covered
r[molten.external_live_pilot_soak.positive_workflow] Molten SHOULD provide a complete positive pilot workflow fixture or operator-managed evidence run that binds node-control, service exchange, blob-ref job, coordination, retention/readback, replay, diagnostics, resource, and rollback child evidence.

#### Scenario: Positive workflow binds required children
- GIVEN the positive pilot workflow evidence set
- WHEN pilot validation inspects required child refs
- THEN each required child class is present and scoped to the pilot workload.

### Requirement: External pilot negative denials are covered
r[molten.external_live_pilot_soak.negative_denials] Molten SHOULD test or record denial evidence for missing peer admission, missing authority, stale ticket, failed replay, diagnostics outside threshold, resource breach, missing retention review, and over-broad pilot scope.

#### Scenario: Missing peer admission denies
- GIVEN pilot evidence without a current peer admission receipt
- WHEN the pilot decision validator runs
- THEN the decision is `deny`
- AND diagnostics identify missing peer admission evidence.

### Requirement: External pilot decision denies missing boundary evidence
r[molten.external_live_pilot_soak.decision_receipt] External pilot decisions MUST deny when peer admission, authority, policy, resource, provenance, source-gate, replay, retention review, diagnostics, rollback, or freshness evidence required by the pilot scope is missing, stale, failed, or mismatched.

#### Scenario: Missing authority denies pilot decision
- GIVEN a live workflow bundle with transport evidence and no matching authority grant
- WHEN the external pilot decision evaluates the bundle
- THEN the decision is `deny`
- AND diagnostics state that transport evidence does not grant authority.

### Requirement: External pilot readback preserves caveats
r[molten.external_live_pilot_soak.release_readback] Operator and release readback for external pilot evidence MUST render pilot caveats and MUST NOT present constrained pilot evidence as broad production readiness.

#### Scenario: Pilot summary cannot override caveats
- GIVEN a passing constrained pilot decision
- WHEN release readback renders the pilot summary
- THEN the summary names the allowed and denied scopes
- AND it states that subsystem gates remain independently required.


### Requirement: Operator dogfood and production workflows are integration shells
r[molten.operator_workflow.modularity.integration_boundary] Operator dogfood, production soak, and NixOS VM workflows SHOULD be owned by integration-shell modules that consume stable runtime APIs and emit canonical review evidence.

#### Scenario: Dogfood workflow consumes stable runtime API
- GIVEN a dogfood workflow exercises node, retention, job, or transport behavior
- WHEN reviewers inspect the implementation
- THEN the workflow calls stable runtime or CLI-adapter APIs and packages resulting evidence rather than being imported by runtime cores

### Requirement: Runtime cores do not depend on operator integration modules
r[molten.operator_workflow.modularity.dependency_direction] Runtime, node, storage, transport, and policy cores MUST NOT import operator dogfood, production soak, or NixOS VM modules.

#### Scenario: Runtime-to-dogfood import is blocked
- GIVEN a runtime core imports an operator dogfood, prod-soak, or NixOS VM module
- WHEN dependency-boundary validation runs
- THEN validation fails or records the violation before release evidence is promoted

### Requirement: Operator integration receipts remain evidence only
r[molten.operator_workflow.modularity.evidence_only] Dogfood, production soak, and NixOS VM receipts MUST remain release-review or diagnostic evidence only and MUST NOT grant authority, policy, resource, provenance, retention, execution, transport, or source-gate trust by themselves.

#### Scenario: Complete dogfood evidence is reviewable
- GIVEN a dogfood workflow has complete child receipts and stable refs
- WHEN the operator workflow packages the run
- THEN it emits review evidence binding child refs and caveats without granting runtime authority

#### Scenario: Diagnostic log alone is denied
- GIVEN a VM or soak run has terminal logs but lacks required canonical receipts
- WHEN release-readiness evidence is evaluated
- THEN diagnostic logs alone are insufficient and the evidence is denied or marked unavailable

### Requirement: Operator integration modularity has positive and negative tests
r[molten.operator_workflow.modularity.tests] Operator workflow boundary refactors SHOULD include positive evidence aggregation tests and negative tests for missing child evidence, stale refs, unavailable VM execution, diagnostic-only logs, or overbroad release claims.

#### Scenario: Overbroad production claim is rejected
- GIVEN a dogfood or soak receipt claims broad production readiness without required supporting evidence
- WHEN validation evaluates the receipt
- THEN the claim is rejected or caveated before promotion
