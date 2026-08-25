# Design: Type authority and artifact references

## Context

Molten already models identity and authority as separate canonical records. Runtime admission checks exact fields such as context, subject, delegation, revocation, key, policy, resource, and evidence references.

Inside Rust, several of those values remain exchangeable strings. This weakens the compiler boundary but does not require a wire-format change.

## Decisions

### Decision: Inventory before migration

**Choice:** Produce a bounded inventory of raw references in pure core structs, function signatures, enums, maps, and adapter DTOs. Classify each row as domain core, wire compatibility, display metadata, external protocol, or deferred.

**Rationale:** Molten is large. A reviewed inventory prevents a mechanical wrapper campaign over unrelated text.

### Decision: Use domain-marked reference families

**Choice:** Add a small set of private generic representations whose marker types encode semantic domains. Separate lexical entity IDs from canonical content or evidence refs when their grammars differ.

Provide aliases for principal, node, actor, service, session, context, delegation, revocation, key, policy, resource, evidence, artifact, operation, and receipt domains.

**Rationale:** Shared parsing remains centralized while distinct marker instantiations prevent category exchange.

### Decision: Keep authority out of constructors

**Choice:** Constructors check syntax, bounds, canonical spelling, and domain tags only. Authority admission still evaluates holder, session, scope, caveats, expiry, revocation, key currentness, policy, resources, and evidence.

**Rationale:** A typed reference identifies a category. It does not grant authority.

### Decision: Separate Preserves wire and admitted core models

**Choice:** Preserve existing canonical Preserves records and decode them into wire DTOs. Convert wire values into typed admitted models before pure authority and lifecycle decisions.

Heterogeneous records use closed enums or explicit role-tagged wrappers. Known-role core functions accept exact aliases.

**Rationale:** Preserves remains canonical. Rust gains compile-time separation after admission.

### Decision: Migrate by trust boundary

**Choice:** Migrate in this order:

1. authority contexts and capability proofsets.
2. effect, handler, and node-control admission.
3. artifact binding, provenance, and operation refs.
4. retention, replay, and receipt linkage.

Do not mix semantic operation descriptor logic or artifact retirement logic into the generic reference layer.

**Rationale:** Each stage has focused tests and an observable boundary.

### Decision: Preserve canonical bytes and replay

**Choice:** Capture baseline canonical Preserves bytes and receipt refs for each migration cohort. Require byte-identical projection and successful historical replay after the Rust type change.

**Rationale:** Internal type safety must not fork canonical evidence history.

### Decision: Compile-test category separation

**Choice:** Add compile-fail fixtures for session/context, policy/evidence, delegation/revocation, key/authority, artifact/receipt, operation/resource, and node/principal substitutions.

**Rationale:** These cases directly prove that the compiler blocks wrong-domain calls.

## Functional core and shell boundary

Parsing, typed conversion, admission decisions, canonical projection, and replay reduction remain pure.

Filesystem, network, Iroh, Wasmtime, process, key storage, clock, and CLI effects remain in their existing shells.

## Test design

Positive tests cover valid domain construction, Preserves admission, same-domain calls, current authority decisions, replay, and unchanged canonical receipts.

Negative tests cover malformed or oversized refs, unknown domains, wrong holders, wrong sessions, wrong scopes, expired or revoked evidence, cross-domain wire fields, compile-time substitutions, and authority-by-possession attempts.

## Risks and trade-offs

- A broad migration can conflict with active work. Cohort boundaries and baseline fixtures reduce merge risk.
- Generic types can hide domain names in diagnostics. Public aliases and domain-specific errors keep output clear.
- External protocols can require strings. Their DTOs remain explicit compatibility scopes.
- Typed refs can create false confidence. Existing authority non-claims remain mandatory.

## Claim boundary

The change prevents selected internal category errors and centralizes reference syntax. It does not prove authority, freshness, evidence truth, transport identity, runtime correctness, or release eligibility.
