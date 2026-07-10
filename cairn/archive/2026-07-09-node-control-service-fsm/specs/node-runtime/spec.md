# Node Runtime Delta: Node-Control Service FSM

### Requirement: Node-control service lifecycle is explicit
r[molten.node_runtime.service_fsm_model] Molten MUST model the node-control service lifecycle as a reviewed finite state machine over service state, event, startup evidence, service-lock evidence, supervisor-policy facts, heartbeat/tick facts, shutdown facts, and explicit shell intents.

#### Scenario: Service reaches serving through reviewed transitions
- GIVEN node initialization and startup evidence have passed and no active service lock exists
- WHEN the service acquires a lock and evaluates the serve event
- THEN Molten emits passing service lifecycle evidence
- AND the next state is serving with startup and service-lock refs bound.

#### Scenario: Serve cannot start from missing startup evidence
- GIVEN a state root has no current startup receipt
- WHEN the service serve event is evaluated
- THEN the transition decision is deny
- AND no service lock, live listener, run-loop, or shutdown side effect is authorized.

### Requirement: Node-control service receipts bind lifecycle state
r[molten.node_runtime.service_fsm_receipts] Node-control service-lock, heartbeat, supervisor, service-run, live-listener, run-loop, health, and shutdown receipts MUST bind the relevant service FSM prior state, event, next state or preserved state, startup ref, service-lock ref when present, supervisor policy ref when present, decision, diagnostics, and evidence-only caveats.

#### Scenario: Denied duplicate runner preserves service state
- GIVEN a service lock is already active for a startup receipt
- WHEN a second runner attempts to acquire service ownership
- THEN Molten emits a deny receipt bound to the preserved service state
- AND the existing service lock remains authoritative only as lifecycle evidence, not operation authority.

### Requirement: Stale lock recovery and restart bounds are transitions
r[molten.node_runtime.service_fsm_lock_recovery] Molten MUST model stale-lock recovery, duplicate-runner denial, heartbeat timeout, restart admission, restart denial, shutdown requested, shutdown drain, and drain completion as explicit node-control service FSM transitions that fail closed when policy or evidence is missing.

#### Scenario: Stale lock without recovery policy denies
- GIVEN a stale service lock is observed and no current supervisor policy permits stale-lock recovery
- WHEN recovery is evaluated
- THEN the transition decision is deny
- AND no lock replacement or service restart side effect occurs.

#### Scenario: Shutdown drain completes before stop
- GIVEN a running node-control service receives a shutdown request and pending inbox work is within the reviewed drain bound
- WHEN drain completion is evaluated
- THEN Molten emits passing service lifecycle evidence
- AND the next state permits stop and lock release.

### Requirement: Node-control service FSM tests cover positive and negative paths
r[molten.node_runtime.service_fsm_tests] Molten SHOULD test the node-control service FSM with positive startup, serve, heartbeat, drain, shutdown, and stop traces, and negative traces for missing startup evidence, duplicate runner, stale lock without recovery, stale startup binding, heartbeat timeout, restart bound exhaustion, and shutdown drain over limit.

#### Scenario: Generated service trace rejects illegal restart
- GIVEN a generated node-control service trace exceeds the configured restart bound
- WHEN the supervisor restart event is evaluated
- THEN the transition decision is deny
- AND the state remains unchanged until a reviewed recovery path is supplied.