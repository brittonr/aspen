# Operator Workflow Delta: Nix dogfood release evidence binding

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
