## ADDED Requirements

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
