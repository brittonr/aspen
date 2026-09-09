# System Extension Runtime Delta

## ADDED Requirements

### Requirement: Typed service facades lower to the existing interface
r[molten.system_extension.service_facades] Aspen MUST admit library-level Service, StateMachine, and Task facade profiles over the existing system-extension interface. A facade turn MUST lower `current state plus admitted event` into `next state, replies, effect intents, and timer intents` through the existing canonical envelopes, admission checks, lifecycle transitions, and evidence. Facades MUST NOT add a runtime implementation, I/O route, or state store. State-machine timers and postponement MUST be incarnation- and phase-fenced and explicitly bounded, and a concurrency limit above one MUST NOT permit concurrent mutation of one logical service's state.

#### Scenario: Facade turn equals hand-written turn
- GIVEN the same admitted event against the same state
- WHEN the facade lowers the turn and a hand-written extension produces the same turn
- THEN the canonical envelopes, admission results, and evidence records are identical.

#### Scenario: Illegal event for the phase
- GIVEN a state machine in a phase whose table does not admit the arriving event
- WHEN the event is presented
- THEN the facade rejects it with a typed phase issue and the state does not change.

#### Scenario: Stale-state timer cannot transition a replacement
- GIVEN a state timeout scheduled by an earlier phase or an earlier incarnation
- WHEN the timer fires after a phase change or a restart
- THEN the timer is rejected as stale and cannot cause a transition in the current state or replacement instance.

#### Scenario: Postponement stays bounded
- GIVEN events postponed in a phase up to the admitted bound
- WHEN one more event arrives beyond the bound
- THEN the event dead-letters through the existing delivery path instead of extending the queue.

#### Scenario: Parallel events serialize per service
- GIVEN a service facade configured with a concurrency limit above one
- WHEN two events arrive in parallel for the same logical service state
- THEN they serialize through the controlled commit and no transition observes a partial or interleaved state.
