# Content replication: F07 delta

## ADDED Requirements

### Requirement: Deficits do not depend on actions
r[molten.audit_f07.deficit_status] Molten MUST preserve each unmet replica demand in status independently from the presence of executable actions.

#### Scenario: Satisfied inventory clears demand
- GIVEN current admitted inventory satisfies every replica rule
- WHEN status is projected without transfer actions
- THEN no under-replicated content is reported.

#### Scenario: No target retains demand
- GIVEN two replicas are required, one is verified, and no eligible target exists
- WHEN status is projected from an empty action list
- THEN the content remains under-replicated with an explicit insufficient-peer cause.

### Requirement: Partial success cannot hide residual demand
r[molten.audit_f07.per_content_accounting] Molten MUST resolve deficits only from sufficient current verified evidence for the same content reference.

#### Scenario: Enough verified results resolve demand
- GIVEN admitted results supply every missing replica for one content reference
- WHEN status applies those results
- THEN the deficit for that content clears.

#### Scenario: A successful subset remains partial
- GIVEN selected targets cover only part of the missing replicas
- WHEN all selected transfers succeed
- THEN residual demand remains under-replicated and another content result cannot fill it.

### Requirement: Unresolved demand prevents completion
r[molten.audit_f07.receipt_decision] Molten MUST prevent complete decisions for unresolved demand and MUST preserve durable state on rejection.

#### Scenario: Convergence permits completion
- GIVEN all demand is satisfied and no active operations or failures remain
- WHEN the shell classifies the receipt
- THEN completion is permitted under the existing admission rules.

#### Scenario: No-work deficit remains partial
- GIVEN unresolved demand with no target and no action
- WHEN the shell classifies the receipt
- THEN the receipt is not complete and no transfer effect is fabricated.

#### Scenario: Denial preserves state
- GIVEN invalid planning input or rejected verification
- WHEN the core or adapter rejects the work
- THEN prior durable state remains intact and the receipt does not claim completion.

### Requirement: Evidence retains its execution scope
r[molten.audit_f07.validation_claims] Molten MUST retain repository-owned positive and negative tests and MUST distinguish core observations from shell receipt evidence.

#### Scenario: Normal regression covers the shell
- GIVEN the F07 no-target fixture
- WHEN normal core and controlled-adapter tests run after implementation
- THEN status retains the deficit and the shell emits a non-complete receipt.

#### Scenario: Audit evidence cannot imply live execution
- GIVEN only the original F07 planner and status counterexample
- WHEN a report describes receipt behavior
- THEN it labels receipt classification as static evidence, not an executed live receipt.
