## ADDED Requirements

### Requirement: Owner/public API readiness review [r[trust-crypto-secrets-extraction.owner-public-api-review]]

The trust/crypto/secrets extraction SHALL complete an owner/public API review before promoting the aggregate family beyond `workspace-internal`.

#### Scenario: Review separates reusable and runtime surfaces [r[trust-crypto-secrets-extraction.owner-public-api-review.surface-classification]]

- GIVEN the family includes crypto helpers, trust state, secrets type contracts, secrets implementation services, and handler/runtime adapters
- WHEN readiness is reviewed
- THEN the review SHALL record which crates/features are canonical reusable public API surfaces
- AND it SHALL record which crates/features remain runtime adapters or compatibility consumers.

#### Scenario: Promotion requires real checker coverage [r[trust-crypto-secrets-extraction.owner-public-api-review.checker-coverage]]

- GIVEN the crate-extraction checker is used as readiness evidence
- WHEN the family is considered for `extraction-ready-in-workspace`
- THEN the checker SHALL validate real crate/package candidates for the reusable surfaces instead of deferring direct dependency checks through an aggregate pseudo-candidate.

#### Scenario: Stability contracts are explicit [r[trust-crypto-secrets-extraction.owner-public-api-review.stability-contracts]]

- GIVEN trust and secrets types expose serialized state, custom binary formats, or service DTOs
- WHEN the owner/public API review completes
- THEN it SHALL identify which formats are compatibility contracts and which remain internal implementation details before promotion.
