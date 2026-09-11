# Design: Review the caveat attenuation boundary

## Context

Molten authority flows through policy and capability gates that check scoped allow and deny contexts. Attenuation today
is narrowing by scope: which resources, which evaluation turn, which session. The stack's UCAN dependency owns
delegation and caveats, and Basalt owns policy evaluation over verified grants. The manual's caveat semantics add
rewrite and alternative filters, which no current Molten component expresses.

## Approach

Produce one review decision with four parts.

1. **Need inventory.** For every attenuation site (plugin hostcall grants, remote session tickets, vat proxies, store
   views, project scopes), record the required narrowing and classify it as scope restriction or payload filter.
2. **UCAN inventory.** Read the pinned UCAN and Basalt sources and record which caveat forms exist, whether they can
   express a bounded filter over Preserves payloads, and what evidence a holder needs to verify a chain.
3. **Decision.** Choose one of two outcomes: consume the stack authority format, or admit a bounded Molten filter core.
   A third outcome, "add nothing", is valid when no reviewed need requires filters.
4. **Guard rails.** If a Molten-owned form is admitted, fix the semantics that the manual fixes: reverse-order chain
   evaluation with a written direction, unknown caveat rejects all inputs, unbound template references fail
   validation, and attenuation can only narrow.

The rule that prohibits a second authority format before this decision is normative and stays in the accepted spec
after the change archives.

## Decisions

### Decision: Decide before implementing

**Choice:** Keep this package review-only and record the rule as a requirement.

**Rationale:** The alternative is a second authority format that duplicates UCAN, splits revocation, and doubles the
verification burden. A written decision is cheap and is the input the later implementation package needs.

## Risks / Trade-offs

- A review package produces no runtime behavior. Its evidence is the decision note plus the enforced rule, and the
  non-claim is that no filter exists yet.
- A payload filter over authority-bearing values is itself a security surface. If the decision admits one, the
  follow-up package must carry its own adversarial fixtures, which this package does not provide.
