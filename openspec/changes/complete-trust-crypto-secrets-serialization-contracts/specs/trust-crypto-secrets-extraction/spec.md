## ADDED Requirements

### Requirement: Trust and secrets serialization contract evidence [r[trust-crypto-secrets-extraction.serialization-contract-evidence]]

The trust/crypto/secrets extraction SHALL capture deterministic serialization contract evidence before promoting reusable trust or secrets surfaces beyond `workspace-internal`.

#### Scenario: Stable trust formats have goldens and roundtrips [r[trust-crypto-secrets-extraction.serialization-contract-evidence.trust-goldens]]

- GIVEN `aspen-trust` exposes share, encrypted envelope, encrypted chain, threshold, or trust protocol formats as reusable compatibility contracts
- WHEN serialization evidence is captured
- THEN deterministic golden bytes or JSON fixtures SHALL exist for each stable format
- AND roundtrip tests SHALL prove decoding preserves the contract value
- AND malformed, truncated, or wrong-version bytes SHALL be rejected where the format has an explicit parser.

#### Scenario: Stable secrets-core persisted state has goldens and roundtrips [r[trust-crypto-secrets-extraction.serialization-contract-evidence.secrets-core-goldens]]

- GIVEN `aspen-secrets-core` owns KV, Transit, and PKI DTO/state contracts used by storage or compatibility re-exports
- WHEN serialization evidence is captured
- THEN persisted state and config formats SHALL have deterministic golden/roundtrip coverage
- AND request/response DTOs SHALL be identified as stable compatibility contracts or internal implementation details.

#### Scenario: Serialization contract scope is recorded before promotion [r[trust-crypto-secrets-extraction.serialization-contract-evidence.scope-recorded]]

- GIVEN trust/crypto/secrets readiness is considered for `extraction-ready-in-workspace`
- WHEN verification evidence is reviewed
- THEN the change SHALL record which trust/secrets formats are stable compatibility contracts
- AND it SHALL record which serialized forms remain internal and are excluded from compatibility guarantees.
