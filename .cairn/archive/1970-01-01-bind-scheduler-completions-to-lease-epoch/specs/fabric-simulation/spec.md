# Fabric Simulation Delta

## ADDED Requirements

### Requirement: Completions bind to the exact assignment
r[molten.fabric_simulation.assignment_token] A scheduler completion request MUST carry an assignment token derived from the job, the owner, and the lease epoch of the assignment that produced the work. The transition MUST recompute the expected token from current state and MUST reject any mismatch with a typed stale-assignment error, even when the owner matches.

#### Scenario: Stale completion after ownership returns
- GIVEN job J was leased to worker A at epoch 1, failed over to worker B at epoch 2, and failed over back to worker A at epoch 3
- WHEN worker A submits the completion token from its epoch-1 assignment
- THEN the transition rejects the completion with the stale-assignment error and the job state does not change.

#### Scenario: Current token is accepted
- GIVEN worker A holds the epoch-3 assignment of job J
- WHEN worker A submits the completion token issued at epoch 3
- THEN the transition completes the job and records the authoritative completion reference.

#### Scenario: Token reuse after completion
- GIVEN job J completed under the epoch-3 assignment
- WHEN any worker resubmits a completion with the epoch-3 token
- THEN the transition rejects the completion as a duplicate and the recorded completion reference does not change.

### Requirement: Independent checking detects an accepted stale completion
r[molten.fabric_simulation.stale_completion_check] A simulation checker MUST derive assignment ownership and epoch facts from the recorded transition history and MUST evaluate whether each authoritative completion matches the assignment that owned the job at that logical position. A service-reported invariant pass list MUST NOT establish this property.

#### Scenario: Checker catches a defective transition
- GIVEN a deliberately broken scheduler variant that skips the token check
- WHEN the owner-returns trace executes and the service reports its invariants as passed
- THEN the independent checker reports a stale-completion failure at the recorded logical position.

#### Scenario: Correct history passes independently
- GIVEN a history where every completion token matches its owning assignment
- WHEN the independent checker evaluates the history
- THEN the checker reports pass and the result does not depend on any reported invariant name.
