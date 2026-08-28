# System Extension Runtime Specification Delta

## ADDED Requirements

### Requirement: Every callback input is a canonical message

r[molten.message_boundary.callback_envelope]

Initialize, start, request, message, stream-open, stream-event, timer, health, checkpoint, recover, drain, shutdown, and effect-completion inputs MUST use bounded owned canonical callback envelopes.

A callback kind is a message discriminant and MUST NOT grant access to a live transport, process, executor, storage, clock, or channel handle.

#### Scenario: Timer callback uses an explicit event

- GIVEN a generation-fenced timer becomes eligible at virtual or admitted live time
- WHEN the extension callback runs
- THEN it MUST receive a canonical timer message with the required timer, generation, logical-time, and deadline facts.

#### Scenario: Stream callback carries a live object

- GIVEN a stream callback value contains a connection, stream, borrowed buffer, or executor handle
- WHEN callback admission runs
- THEN admission MUST fail before extension code executes.

### Requirement: State-owner transitions have explicit inputs and outputs

r[molten.message_boundary.transition_shape]

Each declared system-extension state-owner transition MUST consume explicit current state, an admitted inbound message, validated policy and authority facts, logical time, and other behavior-affecting inputs required by its contract.

It MUST return explicit next state, decisions, events, outbound messages, errors, or typed effect plans without executing external effects.

#### Scenario: Accepted request returns an effect plan

- GIVEN an admitted request message and complete explicit decision facts
- WHEN the pure transition accepts the request
- THEN it MUST return next state and a typed effect plan
- AND the shell MUST execute no effect before authority and resource admission.

#### Scenario: Transition reads adapter liveness

- GIVEN a transition attempts to inspect a live connection or adapter-local health value that is absent from the inbound message
- WHEN architecture admission runs
- THEN admission MUST block the hidden input.

### Requirement: Effect completions re-enter through messages

r[molten.message_boundary.effect_completion]

Every external effect result MUST re-enter the owning state machine as a canonical generation-fenced completion message with explicit success, denial, uncertainty, cancellation, timeout, or infrastructure-failure classification.

A provider callback MUST NOT mutate extension semantic state directly.

#### Scenario: Storage operation completes

- GIVEN the shell executes an admitted durable-state effect plan
- WHEN the provider reports completion
- THEN the shell MUST construct a canonical completion message
- AND the state owner MUST decide the semantic result through its normal transition path.

#### Scenario: Provider mutates extension state

- GIVEN a provider receives mutable access to extension state and applies its result directly
- WHEN boundary validation runs
- THEN validation MUST fail because the completion bypasses message delivery and transition authority.

### Requirement: Active roadmap changes preserve the boundary

r[molten.message_boundary.roadmap_compatibility]

Before this change archives, every active change that introduces connection wake, stream callbacks, retries, sessions, shared state, or new adapters MUST record conformance with the message boundary or receive a narrow compatibility update.

#### Scenario: Addressable actor wakes on connection activity

- GIVEN an addressable actor profile declares connection activity as a wake reason
- WHEN roadmap compatibility review runs
- THEN the wake MUST be represented by a canonical transport or lifecycle message
- AND the actor core MUST NOT inspect a live connection object.

#### Scenario: Active change has an unresolved bypass

- GIVEN an active runtime change cannot express required behavior through admitted messages and effects
- WHEN this change reaches closeout
- THEN closeout MUST block and identify the exact missing primitive or contract.
