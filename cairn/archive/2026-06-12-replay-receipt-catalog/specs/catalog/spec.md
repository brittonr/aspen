# Catalog Delta: replay receipt catalog

### Requirement: Replay receipts have ledger kinds
r[molten.determinism.replay_receipt_catalog.ledger_kind] The evidence ledger SHOULD classify `deterministic-replay-verify-v1` and `deterministic-first-divergence-v1` records with stable artifact kinds.

#### Scenario: Replay verify import is kinded
- GIVEN a generic replay verification receipt
- WHEN it is imported into the evidence ledger
- THEN the ledger artifact kind is `deterministic-replay-verify-receipt`

### Requirement: Replay verification records are catalog-searchable
r[molten.determinism.replay_receipt_catalog.classify_verify] The catalog SHOULD classify replay verification records by decision, divergence kind, expected/actual report refs when present, and state or output refs when present.

#### Scenario: Replay verify is found by final state
- GIVEN an imported replay verification receipt with expected report, actual report, and final-state refs
- WHEN catalog search filters by replay decision and final-state ref
- THEN the replay verification receipt is returned

### Requirement: First-divergence records are catalog-searchable
r[molten.determinism.replay_receipt_catalog.classify_divergence] The catalog SHOULD classify first-divergence records by divergence kind, actor/session/vat identifier when present, handler profile ref, expected ref, and actual ref.

#### Scenario: Divergence is found by kind
- GIVEN an imported deterministic first-divergence record for an effect-response mismatch
- WHEN catalog search filters by `replay-divergence:effect-response`
- THEN the first-divergence record is returned

### Requirement: Replay receipt catalog coverage is tested
r[molten.determinism.replay_receipt_catalog.tests] Molten SHOULD test ledger import and catalog search for generic replay verification and first-divergence evidence.

#### Scenario: Search returns replay evidence only
- GIVEN imported replay verification and first-divergence records
- WHEN catalog searches by replay decision, divergence, report refs, or final-state refs
- THEN matching replay evidence is returned without granting authority or replacing gate validation
