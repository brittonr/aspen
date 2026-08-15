# Design: Collaboration-scope system extension

## Context

Molten fabric membership records node observations, placement, service roles, and fencing. Collaboration membership records application subjects and their relationship to shared resources. Combining these domains would let transport or placement facts cross an authority boundary.

Basalt evaluates exact policy, resource, ability, request, caveat, revocation, and replay facts. UCAN verifies bounded delegation facts. Neither component owns durable collaboration membership or resource-to-scope relationships.

## Success Contract

Completion requires a versioned extension, nominal references, pure transitions, current membership and policy binding, non-transitive sharing, consistency-backed mutation, safe snapshots, paired failure tests, and explicit non-claims. A table of group names, a UI participant list, or a valid UCAN token without current scope facts is false completion.

## Architecture

```text
scope request and current state
  -> pure shape and transition validation
  -> shell-owned current authority and policy fact resolution
  -> pure exact-binding and effective-audience decision
  -> consistency and durable-state commit
  -> canonical receipt and metadata-only snapshot
  -> authorized downstream consumer
```

The pure core receives only loaded values. It performs no filesystem, network, process, clock, transport, key, token, policy-loading, or persistence effects.

The imperative shell resolves current state through admitted fabric ports. It supplies verified Basalt and UCAN decision facts, proposes mutations through consistency, persists accepted state, and emits bounded evidence.

## Canonical Model

### Scope records

A `CollaborationScope` binds these fields:

- nominal scope reference and closed scope-kind profile;
- owner subject reference;
- scope revision and membership epoch;
- optional organization-floor policy reference;
- current scope and resource-policy references;
- bounded active membership records;
- lifecycle state and canonical state reference.

The first closed profile contains personal, conversation, group, and organization kinds. Unknown kinds fail closed. A personal scope admits only its owner unless a later reviewed profile defines another rule.

### Membership records

A membership record binds a scope, subject, role, lifecycle state, admitted abilities, start epoch, optional logical expiry, authority-decision references, and operation identity. Display names, email addresses, transport identities, and fabric node identities are not subject references.

Membership mutation increments the membership epoch. Removal affects later admissions. It does not claim to erase prior disclosures or cancel an external effect already in progress.

### Resource bindings

A `ScopedResourceBinding` binds one resource reference to one owner scope, owner subject, binding revision, resource-policy reference, required abilities, and evidence references. Resource bytes remain in the owning product or admitted content store.

A move requires current source and target scope state, explicit move authority, target admission, and one atomic or consistency-serialized transition. A read grant, matching name, parent scope, or destination membership cannot create a copy or move.

## Effective Audience

An audience request names the exact scope, scope revision, membership epoch, resource binding, requested subjects, ability, request identity, policy references, and current decision references.

The pure reducer admits the audience only when:

- every selected subject has current active membership;
- the requested ability passes scope and resource policy;
- the organization floor, when present, also permits the request;
- all Basalt and UCAN decision inputs match the same request and current state;
- no supplied revocation, expiry, replay, or consistency fact invalidates the request.

Effective authority is an intersection. A narrower scope can reduce access, but it cannot weaken an organization floor. Missing facts deny rather than inherit access from another scope.

## Sharing Is Non-Transitive

`resource/read`, `resource/use`, and product-specific abilities do not imply `resource/share`. Resharing requires a separate exact request and current decision for the target scope.

A resource grant never makes its recipient the resource owner. Scope ancestry, shared participants, prior delivery, or UI visibility never grants a new binding ability.

## Admission Snapshots

A `ScopeAdmissionSnapshot` contains only:

- scope, scope revision, and membership epoch references;
- resource-binding and audience-set references;
- requested ability and request identity;
- policy, authority-decision, consent-decision, and revocation-observation references;
- consistency epoch, logical-currentness facts, outcome, and safe diagnostics;
- BLAKE3 state and receipt identities.

A snapshot is evidence for one request. It is not a bearer capability, reusable token, membership store, policy source, or guarantee of future authority. Consumers must revalidate currentness before a protected effect.

Raw UCAN tokens, identity documents, membership labels, policy bodies, messages, memories, files, prompts, credentials, and resource bytes are prohibited from snapshots and receipts.

## Consumer Boundary

- Lattice may bind a current snapshot to an exact effect attempt and must revalidate before dispatch.
- Animus may receive selected scope and resource references in context metadata. It does not store membership or decide access.
- Tile may render bounded scope status. It does not create membership, policy, or authority facts.
- Other consumers must declare equivalent authority and currentness boundaries.

A workspace-relative sibling checkout is test input only. Production consumers need immutable schema and evidence identities.

## Consistency, Time, and Recovery

Scope and membership mutations use the admitted consistency service and durable-state profile. Receipts bind operation identity, current engine epoch, before and after state references, and durability outcome.

Logical expiry uses admitted fabric logical time. A process timer or local stale read cannot expire membership or admit a snapshot by itself.

Duplicate identical operations are idempotent. Conflicting duplicate operations, stale epochs, stale generations, foreign owners, and uncertain durability deny without inventing success.

## Functional Core and Imperative Shell

### Functional core

- nominal reference and bounded record validation;
- scope, membership, binding, move, and snapshot transitions;
- effective-audience intersection;
- organization-floor composition;
- non-transitive sharing checks;
- deterministic diagnostics and receipt payloads.

### Imperative shell

- Preserves decoding and canonical encoding;
- system-extension lifecycle and port binding;
- consistency proposals and durable persistence;
- logical-time and currentness acquisition;
- Basalt and UCAN decision acquisition;
- operator status and evidence publication.

## Failure Behavior

The extension fails closed for unknown scope kinds, malformed references, empty or duplicate member sets, over-bound collections, stale epochs, foreign operations, missing policies, wrong abilities, revoked decisions, replayed requests, expired memberships, implicit scope moves, transitive shares, payload bytes, and unsafe diagnostics.

Availability loss can block protected access. The extension must report that state without treating cached allowance as fresh authority.

## Validation

Positive fixtures cover each scope kind, member add and remove, exact resource binding, authorized move, effective audience admission, organization-floor narrowing, and current downstream snapshot use.

Negative fixtures cover stale membership, one denied audience member, wrong scope, wrong resource, wrong ability, weakened organization floor, transitive sharing, implicit copy, replay, revocation, expiry, partition, restart, conflicting operations, payload leakage, and stale consumer use.

Property and simulation tests cover deterministic replay, membership-epoch monotonicity, one-owner-scope invariants, denial state preservation, and convergence after admitted current commits.

## Risks and Trade-offs

- Large audiences can make admission expensive. Policy must bound member and decision collections.
- Fail-closed currentness can reduce availability during partitions. The extension must not hide that trade-off.
- Prior allowed disclosure cannot be recalled. Receipts must keep revocation claims prospective.
- Product consent rules differ. The extension carries exact consent-decision references but does not define product consent policy.

## Reference and Claim Boundary

QM commit `7f2c916360f1797a8ff2a77ce2ce40c5fabab087` informs the collaboration-scope and non-transitive ACL scenarios only. QM tests were not run, and its implementation is not production evidence for Molten.

Passing this change proves selected scope-model, transition, binding, currentness, and evidence facts under declared inputs. It does not prove identity truth, global revocation freshness, Basalt or UCAN correctness, transport confidentiality, external-effect safety, prior-disclosure erasure, or release eligibility.
