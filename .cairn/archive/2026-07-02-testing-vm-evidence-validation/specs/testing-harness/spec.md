## ADDED Requirements

### Requirement: VM evidence is semantically validated
r[molten.testing.vm_evidence.semantic_validation] Molten MUST validate NixOS VM evidence by parsing canonical receipt contents, not only by checking marker strings or command success. Validation MUST bind the expected topology, node ids, state roots, Nix store refs, child workflow refs, replay status, diagnostics, decision status, and evidence-only caveats.

#### Scenario: Passing VM evidence validates by content
- GIVEN a completed multi-node VM test with topology, node evidence, VM test-run, and production-soak receipts
- WHEN the VM evidence validator evaluates the canonical receipts against the expected topology
- THEN validation passes only if receipt contents bind the expected nodes, package refs, state roots, child receipt refs, replay status, diagnostics, and pass decision
- AND raw terminal output is not accepted as a substitute for the canonical receipts.

#### Scenario: Marker-only evidence is rejected
- GIVEN VM-local files that contain expected receipt kind strings but omit required topology, child refs, replay status, or decision fields
- WHEN the VM evidence validator evaluates the files
- THEN validation fails closed with diagnostics for the missing semantic bindings.

### Requirement: VM check outputs preserve canonical evidence
r[molten.testing.vm_evidence.artifact_preservation] Molten MUST preserve canonical VM evidence receipts from platform integration checks as explicit Nix output artifacts with a manifest that binds artifact paths, receipt kinds, BLAKE3 content refs, diagnostic log refs, and evidence-only caveats.

#### Scenario: VM check output contains reviewable evidence
- GIVEN a successful `nixos-vm-multinode` check
- WHEN an operator inspects the realized Nix output path
- THEN the output contains a manifest plus the canonical topology, node evidence, VM test-run, production-soak, and child evidence receipts needed for review
- AND each manifest entry binds a stable content ref and receipt kind.

#### Scenario: Empty VM output cannot satisfy release evidence
- GIVEN a VM test derivation that completes but does not preserve canonical evidence artifacts
- WHEN release evidence validation evaluates the derivation output
- THEN the output is denied or marked unavailable for release-evidence purposes even if the build log contains passing assertions.

### Requirement: VM logs remain diagnostic evidence
r[molten.testing.vm_evidence.log_boundary] VM terminal output, QEMU logs, systemd journals, and rendered summaries MUST be treated as diagnostic evidence only. They MAY be preserved and referenced by the VM evidence manifest, but they MUST NOT replace canonical receipt validation for pass evidence.

#### Scenario: Log text cannot override a deny receipt
- GIVEN preserved VM logs that contain successful-looking text and a canonical VM test-run receipt with a deny decision
- WHEN VM evidence validation runs
- THEN validation follows the canonical deny receipt
- AND the successful-looking log text remains diagnostic-only.

### Requirement: VM semantic validation has negative fixtures
r[molten.testing.vm_evidence.negative_fixtures] Molten SHOULD test VM evidence validation with negative fixtures covering missing receipts, stale refs, tampered receipt bytes, wrong topology membership, wrong decision status, incomplete child refs, missing replay status, and unbound diagnostic logs.

#### Scenario: Tampered VM evidence fails closed
- GIVEN a previously passing VM evidence bundle whose node evidence or child receipt ref has been changed
- WHEN the semantic validator evaluates the bundle
- THEN validation fails closed before the bundle can satisfy release or pilot evidence review.

### Requirement: VM evidence inspection is documented
r[molten.testing.vm_evidence.docs] User-facing documentation SHOULD explain which VM output artifacts are authoritative, how to inspect the manifest and canonical receipts, and why logs are diagnostic-only.

#### Scenario: Operator follows VM evidence docs
- GIVEN an operator reviewing a realized VM check output
- WHEN they follow the documented inspection procedure
- THEN they can identify the authoritative VM receipts, their content refs, the validation decision, child workflow evidence, and diagnostic log refs without relying on raw build-log scraping.
