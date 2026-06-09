## ADDED Requirements

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
