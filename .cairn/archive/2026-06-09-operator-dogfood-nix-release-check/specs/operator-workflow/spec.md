# Operator Workflow Delta: Nix Dogfood Release Check

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
