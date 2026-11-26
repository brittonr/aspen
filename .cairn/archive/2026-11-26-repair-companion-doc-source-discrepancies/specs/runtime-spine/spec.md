# Runtime spine deltas

## ADDED Requirements

### Requirement: Snapshot receipt check labels name the evaluated predicate
r[molten.runtime_spine.snapshot_receipt_check_label] A snapshot receipt check label MUST name the predicate the validator evaluates. The readable-assertions check compares readable assertions against admitted references, so it MUST be labelled `readable-assertions-subset-admitted` in a versioned receipt. Existing receipts keep their canonical identity.

#### Scenario: Receipt label matches the admitted-set predicate
- GIVEN a snapshot whose readable assertions are a subset of its admitted references
- WHEN the snapshot evaluator emits its receipt
- THEN the passing check MUST be labelled `readable-assertions-subset-admitted`.

#### Scenario: Assertion outside the admitted set fails the check
- GIVEN a snapshot with a readable assertion that is not an admitted reference
- WHEN the snapshot evaluator emits its receipt
- THEN the `readable-assertions-subset-admitted` check MUST fail.

### Requirement: Capability contexts deny by default
r[molten.runtime_spine.capability_context_deny_by_default] The default capability context MUST grant nothing. Granting every request MUST require an explicit, named allow-all constructor and MUST NOT be reachable through `Default`.

#### Scenario: Default context denies
- GIVEN a capability context created with `CapabilityContext::default()`
- WHEN any actor requests any capability
- THEN authorization MUST deny the request.

#### Scenario: Explicit grant still admits
- GIVEN a capability context built from an explicit grant that matches the request
- WHEN the actor requests that capability
- THEN authorization MUST admit the request.
