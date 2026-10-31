# System Extension Node Hosting Delta

## ADDED Requirements

### Requirement: Public node hosts admitted recovery groups durably
r[molten.system_extension.node_hosting] The public node MUST expose admitted installation, status, supervision, durable-work submission/readmission/delivery and generation upgrade operations backed by actual native lifecycle execution and capability-rooted durable state. Group restart history, current incarnation/generation fencing, durable logical work and current delivery claims MUST survive actual node process loss. Pure planners MUST remain effect-free. Source/startup admission MUST precede lifecycle effects. Interrupted effects MUST remain uncertain until their durable postconditions are observed, and MUST NOT be blindly repeated.

#### Scenario: Actual crash and readmission
- GIVEN an installed admitted recovery group and retained durable work
- WHEN its public node process is killed and a replacement reopens the same capability-rooted state
- THEN ordered group recovery retains its restart history, stale runtime-local work and old claims are rejected, and durable work acts only after explicit current-claim readmission.

#### Scenario: Restart storm survives process replacement
- GIVEN restart attempts within an admitted logical-time window
- WHEN the node is replaced between attempts
- THEN replacement does not reset the budget and the owning recovery planner terminates or escalates the storm within that window.

#### Scenario: Admitted generation migration
- GIVEN a running member, its exact admitted replacement executable/profile/schema and an independent sibling
- WHEN the node drains, checkpoints, materially transforms state and activates the replacement generation
- THEN every stage has a durable intent and observation, pre-upgrade work cannot act in the replacement, explicit transformed durable work can be readmitted, and the sibling continues independently.

#### Scenario: Interrupted lifecycle effect
- GIVEN a persisted intent without a committed terminal observation
- WHEN the node reopens after process loss
- THEN it observes the real native journal, advances only for the exact intended postcondition and otherwise retains an operator-visible uncertain/fenced state without repeating the callback.
