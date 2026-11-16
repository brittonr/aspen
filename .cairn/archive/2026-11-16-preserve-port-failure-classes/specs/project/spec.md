# Project Delta

## MODIFIED Requirements

### Requirement: Fabric port errors are typed

r[molten.modularity.fabric_boundary.errors] Maintained fabric ports MUST distinguish domain rejection, capability failure, timeout, cancellation, storage failure, transport failure, and uncertain external outcome where applicable. Converting a port error into the shared failure type MUST keep its class, and shells MUST select behavior by that class rather than by message text.

#### Scenario: Port returns a raw string failure

r[molten.modularity.fabric_boundary.enforcement.raw_error]
- GIVEN a maintained fabric port returns `String` or another untyped text error
- WHEN architecture validation runs
- THEN validation MUST fail before closeout.

#### Scenario: Port failure crosses the shared failure type

- GIVEN a fabric port error of any class, including uncertain external outcome
- WHEN it is converted into the shared failure type and back
- THEN the class and message are unchanged
- AND a harness failure with the same text is not treated as that class.
