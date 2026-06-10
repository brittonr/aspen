## ADDED Requirements

### Requirement: Release promotion gate receipt
r[molten.evidence.release_promotion.receipt] Molten MUST emit a canonical `release-promotion-gate-receipt-v1` that aggregates release bundle verification, signed keyring currentness, source evidence, Octet evidence, Cairn evidence, diagnostics, and evidence-only caveats.

#### Scenario: Promotion receipt passes for complete evidence graph
- GIVEN a passing release evidence bundle verification receipt
- AND a current unrevoked signed receipt key selected from the keyring
- AND source, Octet, and Cairn evidence markers
- WHEN release promotion verification runs
- THEN Molten emits a `release-promotion-gate-receipt-v1` with decision `pass`
- AND the receipt binds the bundle verify ref, selected key ref, source evidence ref, Octet evidence ref, and Cairn evidence ref

### Requirement: Release promotion CLI
r[molten.evidence.release_promotion.cli] Molten MUST expose `molten dogfood release-promote` to create release promotion receipts from a realized dogfood output, bundle verification receipt, signed keyring ledger, and explicit source/Octet/Cairn evidence markers.

#### Scenario: CLI writes pass or deny receipt
- GIVEN promotion inputs that are complete or incomplete
- WHEN the CLI runs
- THEN it writes a canonical promotion receipt to the requested output path
- AND denial diagnostics are stored in the receipt rather than only in logs

### Requirement: Promotion binds signed keyring currentness
r[molten.evidence.release_promotion.keyring] Release promotion MUST fail closed when the selected signed receipt key is missing, ambiguous, stale, revoked, scoped to the wrong signer, or scoped to the wrong trust root.

#### Scenario: Revoked key denies promotion
- GIVEN a release bundle verification receipt that otherwise passes
- AND a keyring ledger containing a revocation record for the selected signed receipt key
- WHEN release promotion runs with that key selected
- THEN the promotion receipt decision is `deny`
- AND diagnostics identify the signed keyring currentness failure

### Requirement: Promotion binds bundle verification and output path
r[molten.evidence.release_promotion.bundle] Release promotion MUST bind the release bundle verification receipt and the realized output path ref and MUST deny promotion when the bundle verification receipt is not passing or was produced for a different output path.

#### Scenario: Stale bundle verification denies promotion
- GIVEN a bundle verification receipt for one output path
- WHEN promotion is run against a different realized output path
- THEN the promotion receipt decision is `deny`
- AND diagnostics identify the output path ref mismatch

### Requirement: Promotion binds source, Octet, and Cairn evidence markers
r[molten.evidence.release_promotion.source_gates] Release promotion MUST bind explicit source, Octet, and Cairn evidence markers as deterministic refs and MUST deny promotion when any required marker is missing.

#### Scenario: Missing source gate marker denies promotion
- GIVEN a passing bundle verification receipt and current signed key
- WHEN the source evidence marker is empty
- THEN the promotion receipt decision is `deny`
- AND diagnostics identify the missing source evidence marker

### Requirement: Release promotion remains evidence only
r[molten.evidence.release_promotion.evidence_only] Release promotion receipts MUST NOT grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation trust, or permission to bypass subsystem gates.

#### Scenario: Promotion pass does not replace subsystem gates
- GIVEN a passing release promotion receipt
- WHEN a subsystem performs privileged, destructive, transport, provenance-sensitive, source-gated, or retention-sensitive work
- THEN it still requires its own matching gate receipts and MUST NOT treat the promotion receipt as subsystem authority
