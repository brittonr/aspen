# Runtime Patterns Delta: Sans-IO Protocol Cores

### Requirement: Protocol state machines are Sans-IO cores
r[molten.runtime_patterns.sans_io_protocol_core] Molten-owned protocol state machines that control replayable runtime, node-control, peer, transport, job, or artifact-sync behavior MUST keep semantic transition logic in deterministic Sans-IO cores that perform no filesystem, network, database, process, clock, random, async runtime, tracing, stdout/stderr, or receipt-storage effects.

#### Scenario: Same inputs produce same transition
- GIVEN a protocol core receives the same prior state, canonical message ref, deterministic freshness facts, limit profile, and admission facts
- WHEN the transition is evaluated twice
- THEN both evaluations produce the same decision, state delta, outbound envelope refs, effect intents, and diagnostics.

#### Scenario: Ambient clock cannot decide transition
- GIVEN a protocol transition depends on freshness
- WHEN the core is evaluated
- THEN freshness is supplied as an explicit input fact rather than read from a wall clock inside the core.

### Requirement: Protocol inputs are explicit and canonical
r[molten.runtime_patterns.sans_io_explicit_inputs] Protocol cores MUST receive explicit state, event or message, deterministic freshness or sequence facts, limit profiles, authority or policy facts, replay facts, and recorded effect responses whenever those facts can affect the transition decision.

#### Scenario: Missing admission fact denies explicitly
- GIVEN a transition requires authority or policy evidence
- WHEN the caller omits the corresponding fact
- THEN the protocol core returns a deny decision with diagnostics instead of reading ambient authority from the shell.

#### Scenario: Message identity binds canonical input
- GIVEN a protocol core evaluates an incoming message that crosses an evidence boundary
- WHEN it records transition evidence
- THEN the evidence binds the canonical message or envelope ref used as input.

### Requirement: Protocol outputs are effect intents, not effects
r[molten.runtime_patterns.sans_io_transition_outputs] Protocol cores MUST return explicit state deltas, outbound envelope descriptors, effect intents, alarms, diagnostics, and receipt input facts, and MUST NOT send network messages, mutate stores, publish dataspace assertions, write receipts, or emit logs directly.

#### Scenario: Outbound message is returned for shell delivery
- GIVEN a transition wants to contact a peer
- WHEN the core evaluates the transition
- THEN it returns an outbound envelope descriptor for the shell to admit and deliver rather than opening an Iroh stream itself.

#### Scenario: Denied transition has no side effect output
- GIVEN an incoming message is malformed, stale, or illegal for the current state
- WHEN the core evaluates it
- THEN the result denies the transition and contains diagnostics without committed state deltas or side-effecting outputs.

### Requirement: Shell adapters drain core outputs after gates
r[molten.runtime_patterns.sans_io_shell_adapter] Runtime shells MUST perform Iroh sends, Redb writes, dataspace publication, receipt persistence, tracing, and adapter effects only after translating a Sans-IO core result through the normal authority, policy, resource, evidence, and replay gates.

#### Scenario: Shell sends after admission
- GIVEN a core returns an outbound Iroh envelope descriptor
- WHEN the shell admits the descriptor against transport, authority, policy, resource, and replay evidence
- THEN the shell may send the frame and persist the corresponding receipt.

#### Scenario: Shell denial prevents mutation
- GIVEN a core returns a store-write effect intent but resource admission denies it
- WHEN the shell processes the intent
- THEN no store mutation is performed and a deny receipt explains the failed gate.

### Requirement: Sans-IO transitions bind replay evidence
r[molten.runtime_patterns.sans_io_replay_binding] Replayable protocol transitions SHOULD bind input message refs, before-state refs, after-state refs or denial state, outbound envelope refs, effect-intent refs, and admission receipt refs so the same transition can be re-evaluated without live adapter effects.

#### Scenario: Replay compares transition refs
- GIVEN a recorded protocol transition fixture
- WHEN replay re-evaluates the transition
- THEN replay compares canonical input/output/state refs and declared variance rather than rendered logs.

#### Scenario: Effect response is recorded input
- GIVEN a protocol transition depends on an effect response from a prior admitted adapter call
- WHEN the transition is replayed
- THEN the response is supplied as recorded canonical evidence rather than re-running the live adapter.