# Operator Workflow Delta

## ADDED Requirements

### Requirement: Local dogfood source gate is unqualified fixture evidence
r[molten.operator_dogfood_node_workflow.spec.unqualified_fixture_gate] The local dogfood workflow MUST NOT present a passing Octet gate receipt it did not obtain from authenticated source evidence, and MUST NOT install one into an artifact registry. It MAY bind an unqualified fixture gate for node startup and loopback job admission. Every dogfood report, release gate and summary that depends on that gate MUST be labelled as bound to an unqualified fixture source gate.

#### Scenario: Dogfood outputs are labelled
- GIVEN a passing local dogfood run
- WHEN its report, release gate and summary are emitted
- THEN each carries the unqualified fixture source-gate label and no registry holds an Octet gate receipt installed by dogfood

#### Scenario: Real consumers reject the dogfood gate
- GIVEN the dogfood fixture gate
- WHEN a strict Octet source-gate consumer validates it
- THEN the consumer denies it
