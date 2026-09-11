# Runtime spine: record pattern subset delta

## ADDED Requirements

### Requirement: Record patterns match at least the declared structure
r[molten.runtime_spine.record_pattern_subset] Molten MAY admit a bounded record pattern form for dataspace routing that matches declared key positions in increasing Preserves order, succeeds when the candidate carries additional undeclared structure, fails when a declared position is absent or mistyped, denies forms outside the admitted bounds before routing, and MUST NOT affect canonical value identity.

#### Scenario: Extra fields keep matching
- GIVEN an admitted record pattern that declares two key positions
- WHEN a candidate record carries those positions plus one additional field
- THEN the match succeeds and bindings arrive in declared position order.

#### Scenario: Missing declared structure fails
- GIVEN an admitted record pattern that declares a key position
- WHEN the candidate record does not carry that position, or carries a different type there
- THEN the match fails and no binding is produced.

#### Scenario: Unadmitted form denies before routing
- GIVEN a pattern outside the admitted subset, or one over the admission bounds
- WHEN the pattern is admitted for routing
- THEN it denies before it controls any delivery.

#### Scenario: Identity stays exact
- GIVEN a record whose canonical value ref is computed for identity
- WHEN the same record is matched by a record pattern for routing
- THEN the canonical value ref is unchanged by the match.
