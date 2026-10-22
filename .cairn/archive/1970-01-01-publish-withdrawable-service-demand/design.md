# Design: Publish withdrawable service demand

## Context

Demand is currently an input vector on the dependency predicate. The dataspace already owns assertion lifetimes,
retraction, and owner-scope cleanup, and the coordination plane already reflects state as dataspace assertions. Service
lifecycle decisions consume predicate results.

## Approach

Publish demand as an assertion owned by the demanding scope:

- The demand record carries the service ref, the demand kind (dependency-gated or force-run), the demander ref, and the
  operation identity.
- The dependency predicate receives the demand facts derived from the live assertion set, so it stays pure and
  deterministic.
- Withdrawal of the last demand assertion for a service makes shutdown eligible. Shutdown still passes its normal
  admission, resource, and receipt gates.
- Force-run demand bypasses dependency ordering only because the record declares it, and the declaration is visible to
  the same predicate.
- A restart request stays a message. It changes retry or restart state and never becomes a demand.

## Decisions

### Decision: Assertions own demand lifetime

**Choice:** Replace the demand input list with dataspace assertions, and derive the predicate input from them.

**Rationale:** Retraction is already the shutdown signal in this runtime, owner-scope cleanup already removes a dead
demander's facts, and the coordination plane can mirror the same records without a second lifetime model.

### Decision: Deduplicate per owner

**Choice:** One live demand record per owner and service; duplicates from the same owner collapse.

**Rationale:** Demand is a requirement, not a counter. Collapsing per owner keeps withdrawal exact and prevents one
demander from inflating demand strength.

## Risks / Trade-offs

- Existing callers pass demand lists directly. The predicate keeps accepting them for migration and diagnostics, but
  evidence-bearing runs require assertion-derived demand.
- A demand assertion is coordination state, not authority. It cannot start work by itself and proves no provider
  exists.
- Shutdown eligibility must not shorten a service's life while another owner still demands it. The last-owner rule is
  the test target.
