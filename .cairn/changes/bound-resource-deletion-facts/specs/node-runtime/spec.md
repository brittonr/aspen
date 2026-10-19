# Node runtime: bounded resource deletion-gate facts

## ADDED Requirements

### Requirement: Deletion gate facts are admitted before diagnostic construction

r[molten.lifecycle.gc_cleanup_gates.bounded_facts] Molten MUST reject a resource deletion-gate input with more than 64 owner refs, more than 64 finalizers, or more than 256 facts in any cleanup-receipt, live-owner, pin, retention-policy, or deletion-authority collection. It MUST reject any resource, owner, finalizer, cleanup, live-owner, pin, retention, or authority fact longer than 4096 UTF-8 bytes. These checks MUST occur before formatting or cloning any deletion-gate diagnostic. Inputs at the exact count and byte limits MUST retain the same blocker decisions and clearing transitions as smaller inputs. Out-of-bound input MUST NOT yield a deletion-ready or blocked decision.

#### Scenario: Exact-bound pin facts still block cleanup
- GIVEN 256 admitted pin refs and valid deletion authority
- WHEN the deletion gate evaluates the resource
- THEN the decision is blocked with those pins as unresolved blockers.

#### Scenario: Clearing bounded blockers restores readiness
- GIVEN an exact-bound admitted owner or finalizer set and valid deletion authority
- WHEN a live owner or unresolved finalizer clears
- THEN the deletion gate returns deletion-ready without changing the remaining in-bound facts.

#### Scenario: One extra fact is denied before diagnostics
- GIVEN 257 pin refs or 65 owner refs
- WHEN the deletion gate admits the input
- THEN it returns an error before producing any decision or formatting a corresponding diagnostic.

#### Scenario: One oversized fact is denied before formatting
- GIVEN a finalizer or pin fact of 4097 UTF-8 bytes
- WHEN the deletion gate admits the input
- THEN it returns an error before constructing that fact's diagnostic.
