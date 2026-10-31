# Testing Harness Delta

## ADDED Requirements

### Requirement: Service recovery composition proves cross-mechanism properties
r[molten.testing.service_recovery_composition] The harness MUST include a deterministic three-service recovery composition (durable stateful service, dependent worker, independent sibling) that asserts: unaffected services continue during a member's failure and recovery; pre-restart runtime-local events cannot act on a replacement instance; durable work survives restart only through explicit readmission; uncertain external effects surface as unknown outcomes and are not blindly repeated; and restart storms terminate or escalate within the admitted window. Deliberately broken variants of the composition MUST fail the corresponding property, and the live-process twin MUST map its observations to the same assertion vocabulary.

#### Scenario: Broken fencing variant fails
- GIVEN a composition variant with the restart-fencing check removed
- WHEN the delayed pre-restart event scenario runs
- THEN the harness fails the stale-instance property and names the accepted delivery.

#### Scenario: Correct composition passes both tracks
- GIVEN the correct composition and its live-process twin
- WHEN every scenario runs in the deterministic track and the live track
- THEN all five properties hold in both tracks and each property result names the scenarios that exercised it.

#### Scenario: Upgrade inside the composition
- GIVEN the running composition
- WHEN an admitted generation upgrade drains, checkpoints, transforms state, and activates the replacement
- THEN stale runtime-local work from before the upgrade is rejected, durable work is readmitted, and the unaffected sibling continues throughout.
