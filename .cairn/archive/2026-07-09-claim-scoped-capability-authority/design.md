## Context

The current `CapabilityToken`, `CapabilityProofset`, `CapabilityRequest`, UCAN verification, and Basalt authority receipt surfaces are already generic enough to represent claim-scoped authority. A token can bind holder, session, context, resource, ability, scope, attenuation, caveats, expiry, revocation refs, policy refs, resource refs, delegation refs, and evidence refs.

The gap is semantic vocabulary and canonical receipt readback. Operators need to say "friend cluster may attest claim kind K for subject domain S" without creating an ambient trusted-peer bit or bypassing UCAN/Basalt.

## Design

### Data flow

```text
external peer / cluster session
  -> authority-claim-v1
  -> capability request:
       holder/session/context = external issuer context
       resource_ref = claim-domain or subject-selector ref
       ability = claim:attest
       scope = claim kind or claim class
  -> UCAN verification receipt(s)
  -> Basalt enforcement receipt(s)
  -> capability admission receipt
  -> authority-claim-admission-v1
  -> optional downstream gate consumes the admitted claim for an exact purpose
```

The claim and its admission receipt are evidence. A subsystem gate must explicitly choose to consume the admitted claim for one claim kind and subject selector. Claim admission does not grant unrelated authority, provenance, source-gate, transport, retention, execution, promotion, or deployment trust.

### Canonical records

`claim-subject-selector-v1` names a subject domain without assuming one hash algorithm. It should carry:

- selector kind, such as `exact-ref`, `ref-prefix`, `artifact-class`, `namespace`, `schema-id`, `release-channel`, `cluster-id`, or `policy-defined`;
- selector value or referenced predicate/policy;
- optional subject-kind bounds, such as artifact, blob, receipt, schema, package, cluster, or job;
- policy refs and resource refs;
- checks that make wildcard or broad selectors visible.

`authority-claim-v1` records the external statement. It should carry:

- issuer, holder, peer/session, and context refs;
- subject selector ref and optional exact subject refs;
- claim kind and claim value/ref;
- claim evidence refs, policy refs, resource refs, caveats, expiry/freshness, and revocation refs;
- checks that classify it as evidence-only until admitted.

`authority-claim-admission-v1` is local admission evidence. It should carry:

- decision and diagnostics;
- claim ref and subject selector ref;
- requested holder/session/context/resource/ability/scope;
- capability admission receipt ref;
- UCAN verification receipt refs and derived grant refs;
- Basalt enforcement receipt refs;
- local policy, resource, freshness, and revocation refs;
- downstream claim-use caveats.

### Capability profile

Use existing capability matching. The recommended profile is:

```text
token_kind   = external-claim-authority
ability      = claim:attest
resource_ref = claim-subject-selector-v1 ref or claim-domain-v1 ref
scope        = claim kind, claim class, or exact claim namespace
```

A narrower ability such as `class:attest`, `release:attest`, or `provenance:attest` may be supported as aliases or typed profiles only if they still compile down to exact capability admission over holder/session/context/resource/scope.

### Functional core and shell split

The pure core validates claim records, selector records, request construction, admission receipt bindings, and downstream claim-use decisions from in-memory canonical values. It returns pass/deny diagnostics and never reads ledgers, discovers peers, contacts clusters, verifies network state, or evaluates Nickel at runtime.

The shell imports artifacts, resolves refs from ledgers, invokes UCAN/Basalt verification, writes receipts, updates indexes, and renders CLI/operator diagnostics.

### Registry and catalog behavior

Ledger presence, registry classification, catalog search, or MCP discovery may surface claim and admission artifacts for operators. Discovery remains read-only evidence and cannot admit a claim without the linked capability admission and UCAN/Basalt receipts.

### Non-goals

- No global trusted peer or trusted cluster flag.
- No hash-specific trust model; BLAKE3 refs are one possible subject domain.
- No authority from Iroh endpoint identity, topic membership, neighbor observations, send receipts, peer sessions, or handoff bundles.
- No replacement for provenance, source-gate, retention, execution, release, or deployment gates.
- No production fallback to local fixture grants when UCAN/Basalt-backed authority is required.
