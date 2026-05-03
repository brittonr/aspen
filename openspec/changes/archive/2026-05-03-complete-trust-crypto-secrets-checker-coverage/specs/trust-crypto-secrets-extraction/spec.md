## MODIFIED Requirements

### Requirement: Owner/public API readiness review [r[trust-crypto-secrets-extraction.owner-public-api-review]]

The trust/crypto/secrets extraction SHALL complete an owner/public API review before promoting the aggregate family beyond `workspace-internal`.

#### Scenario: Promotion requires real checker coverage [r[trust-crypto-secrets-extraction.owner-public-api-review.checker-coverage]]

- GIVEN the crate-extraction checker is used as readiness evidence
- WHEN the family is considered for `extraction-ready-in-workspace`
- THEN the checker SHALL validate real crate/package candidates for the reusable surfaces instead of deferring direct dependency checks through an aggregate pseudo-candidate
- AND the reusable package candidates SHALL match the owner/public API review's canonical reusable surfaces until additional surfaces are explicitly promoted.
