## Context

`sam-service-supervision-runtime` describes the full local service layer, but a narrow record/ledger slice lets implementation proceed with stable DTOs and catalog visibility before any scheduler or actor execution changes. Service evidence must remain canonical Preserves values with Blake3 refs; rendered summaries are convenience views only.

## Goals

- Define `service-manifest-v1` with explicit service id, owner authority, actor/artifact target, dependency refs, provided assertion refs, restart policy, policy refs, resource refs, effect profile refs, and checks.
- Define `service-demand-v1`, `service-status-v1`, `service-lifecycle-receipt-v1`, `service-supervisor-v1`, `service-restart-policy-v1`, and `service-cleanup-receipt-v1` records.
- Provide parse/render helpers that accept only known schema tags and deterministic field order.
- Classify service records in ledger, artifact registry, catalog, and MCP views.
- Render concise summaries from canonical records without expanding hidden/secret refs or trusting text as pass evidence.

## Non-Goals

- No demand observation or service startup.
- No actor execution, OS process supervision, or remote discovery.
- No unbounded dependency graph traversal.
- No service authority derived from a human-readable service name alone.
- No plaintext secret expansion in rendered summaries.

## Records

```preserves
<service-manifest-v1 "molten.service.manifest.v1"
  <service-id "svc:example">
  <owner <authority-context-ref>>
  <target <actor-or-artifact-ref>>
  <requires [<service-id> ...]>
  <provides [<assertion-pattern-ref> ...]>
  <restart-policy <restart-policy-ref>>
  <policy [<policy-ref> ...]>
  <resource [<resource-ref> ...]>
  <effect-profile [<effect-profile-ref> ...]>
  <checks [<check "explicit-authority" "pass"> ...]>>
```

```preserves
<service-lifecycle-receipt-v1 "molten.service.lifecycle-receipt.v1"
  <operation "declare"|"demand"|"status"|"start"|"ready"|"fail"|"restart"|"stop"|"cleanup">
  <decision "pass"|"deny"|"diagnostic">
  <service <service-id>>
  <manifest <manifest-ref-or-none>>
  <status <status-ref-or-none>>
  <authority [<authority-receipt-ref> ...]>
  <resource [<resource-receipt-ref> ...]>
  <effect-profile [<effect-profile-ref> ...]>
  <diagnostics ["..." ...]>
  <checks [<check "schema-known" "pass"> ...]>>
```

`service-status-v1` records include service id, state, manifest ref, demand refs, dependency refs, readiness assertion refs, failure refs, restart counters, monitor refs, and replay identity refs. `service-cleanup-receipt-v1` records include owned assertion refs, observer refs, live-ref refs, pending effect refs, retraction refs, diagnostics, and checks.

## Ledger and Catalog

Ledger classification must identify service manifests, demands, status records, lifecycle receipts, supervisor records, restart policies, and cleanup receipts. Catalog and MCP views may show service ids, states, dependency ids, and receipt refs, but hidden refs and sensitive payload markers must remain redacted.

## Implementation Notes

- Keep DTO construction in pure helpers that return canonical Preserves values.
- Use explicit bounded vectors for dependencies, provided assertions, policy refs, resource refs, effect refs, diagnostics, and checks.
- Prefer prefix-free input structs for helpers to avoid reintroducing high-arity Octet findings.
- Treat unknown service record schema tags as denial diagnostics, not as pass evidence.
