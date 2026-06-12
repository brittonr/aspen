# Runtime Spine Delta: vat generic replay receipts

### Requirement: Vat replay binds generic verify evidence
r[molten.determinism.vat_generic_replay.bind_verify] The vat replay fixture SHOULD include generic `deterministic-replay-verify-v1` pass evidence in addition to vat-local replay receipts.

#### Scenario: Vat replay includes generic pass receipt
- GIVEN the vat replay fixture runs an unchanged replay scenario
- WHEN the fixture artifact is emitted
- THEN it includes a generic deterministic replay verification receipt with a pass decision
- AND the generic receipt ref is available in fixture diagnostics or embedded evidence

### Requirement: Vat replay binds generic first-divergence evidence
r[molten.determinism.vat_generic_replay.bind_divergence] The vat replay fixture SHOULD include generic first-divergence denial evidence for at least one mismatched boundary.

#### Scenario: Vat replay includes generic divergence receipt
- GIVEN the vat replay fixture includes a changed effect response or equivalent replay mismatch
- WHEN the fixture artifact is emitted
- THEN it includes a generic deterministic replay verification receipt with a deny decision
- AND it includes the corresponding `deterministic-first-divergence-v1` value when available

### Requirement: Vat-local replay evidence remains available
r[molten.determinism.vat_generic_replay.keep_vat_local] The vat replay fixture MUST preserve existing vat-local replay receipts while adding generic replay evidence.

#### Scenario: Existing vat replay tooling still sees vat receipts
- GIVEN existing tooling searches for `vat-replay-receipt-v1`
- WHEN the vat replay fixture is emitted after generic replay integration
- THEN the vat-local receipts remain present and canonical

### Requirement: Vat generic replay integration is tested
r[molten.determinism.vat_generic_replay.tests] Molten SHOULD test that vat replay fixture output contains generic pass, denial, and first-divergence records without treating those records as authority.

#### Scenario: Generic records are evidence-only
- GIVEN a vat replay fixture with generic replay verification evidence
- WHEN tests inspect the output
- THEN they find generic pass and denial evidence
- AND the fixture still states that replay evidence is evidence-only rather than authority or trust
