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

