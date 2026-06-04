## ADDED Requirements

### Requirement: Service records are canonical evidence
r[molten.sam_service_records_ledger.spec.canonical_records] Service manifests, demands, statuses, supervisors, restart policies, lifecycle receipts, and cleanup receipts MUST be represented as canonical Preserves records with stable Blake3 refs before they are used as runtime evidence.

#### Scenario: Manifest ref is stable
- GIVEN two byte-identical `service-manifest-v1` records with the same authority, target, dependencies, provided assertions, policy, resource, and effect refs
- WHEN Molten canonicalizes each record
- THEN both records produce the same service manifest ref
- AND the ref can be used by later service lifecycle receipts

#### Scenario: Malformed record denies
- GIVEN a service record with an unknown schema tag or missing explicit owner authority
- WHEN Molten parses the record for service admission
- THEN parsing denies with deterministic diagnostics
- AND the record cannot satisfy service pass evidence

### Requirement: Service manifests carry explicit authority and resource boundaries
r[molten.sam_service_records_ledger.spec.explicit_boundaries] A `service-manifest-v1` MUST bind explicit owner authority, policy, resource, and effect profile refs; a service name alone MUST NOT grant startup or cleanup authority.

#### Scenario: Name-only service cannot start
- GIVEN a service manifest containing only a human-readable service id and target actor name
- WHEN the service runtime evaluates the manifest
- THEN the manifest is denied before demand startup
- AND no readiness or status assertion is committed

#### Scenario: Boundary refs are preserved
- GIVEN a service manifest with explicit authority, policy, resource, and effect profile refs
- WHEN Molten renders catalog or MCP summaries
- THEN the summaries include safe refs or redacted markers
- AND the underlying canonical record remains the normative evidence

### Requirement: Service artifacts are visible without leaking secrets
r[molten.sam_service_records_ledger.spec.catalog_redaction] Service manifests, status records, lifecycle receipts, and cleanup receipts MUST be classified in ledger/catalog/MCP views, and rendered summaries MUST redact hidden refs and secret/confidential markers by default.

#### Scenario: Service status is summarized safely
- GIVEN a `service-status-v1` with readiness refs and a hidden secret-bearing diagnostic payload
- WHEN the catalog renders the service status
- THEN the summary shows service id, state, dependency ids, and receipt refs
- AND the secret-bearing payload is replaced by a redaction marker

#### Scenario: Text summary is not pass evidence
- GIVEN a rendered service summary that says a service is ready
- WHEN a gate evaluates service readiness evidence
- THEN the summary alone is rejected
- AND the gate requires the canonical status or lifecycle receipt refs
