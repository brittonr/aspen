# Node Runtime Delta: Control Daemon Loop

### Requirement: Control loop is bounded and deterministic
r[molten.node_control_loop.spec.bounded_loop] A node control loop MUST require an explicit state root, require the active startup-bound node control lock, process inbox requests in deterministic order, and stop after a bounded maximum request count.

#### Scenario: Loop drains queued requests
- GIVEN a running node with an active control lock and queued status and shutdown requests
- WHEN `molten node run-loop` is executed with a sufficient request bound
- THEN the requests are dispatched in deterministic inbox order
- AND the loop emits a canonical loop receipt binding the processed request refs and dispatch receipt refs.

### Requirement: Heartbeat and loop receipts are canonical
r[molten.node_control_loop.spec.heartbeat_receipts] Each node control loop run MUST emit canonical heartbeat and loop receipts that bind the current startup receipt, active lock evidence, local loop profile, processed request refs, dispatch receipt refs, diagnostics, and bounded-loop checks.

#### Scenario: Loop evidence is ledger-visible
- GIVEN a node control loop run
- WHEN the node ledger is listed
- THEN `node-control-heartbeat-receipt` and `node-control-loop-receipt` artifacts are present
- AND `molten node show` can summarize those receipts.

### Requirement: Duplicate request refs are idempotent
r[molten.node_control_loop.spec.idempotent_duplicates] Dispatch MUST treat a duplicate canonical request ref with an existing outbox control receipt as idempotent, archive the duplicate request, and return the prior control receipt without repeating side-effecting operation dispatch.

#### Scenario: Duplicate status request returns prior receipt
- GIVEN a status request has already been dispatched successfully
- WHEN the same canonical request is submitted and processed again
- THEN the duplicate dispatch returns the prior control receipt ref
- AND no operation side effects are repeated.

### Requirement: Shutdown stops the loop
r[molten.node_control_loop.spec.shutdown_stops_loop] A node control loop MUST stop after a passing shutdown dispatch removes the active control lock and MUST reject further loop runs until the node is restarted.

#### Scenario: Shutdown request exits loop
- GIVEN a running node with a queued shutdown request
- WHEN the control loop dispatches the shutdown request
- THEN the loop receipt records that the node stopped
- AND a subsequent loop run fails closed because no active lock exists.

### Requirement: Run-loop CLI is available
r[molten.node_control_loop.spec.cli] The CLI MUST expose `molten node run-loop --state-root ... --max-requests ...` and optionally write the canonical loop and heartbeat receipts to operator-selected paths.

#### Scenario: CLI writes loop receipt
- GIVEN a running node with queued control requests
- WHEN the operator runs `molten node run-loop --receipt-out loop.preserves`
- THEN `loop.preserves` contains a `node-control-loop-receipt-v1` artifact.

### Requirement: Loop coverage is present
r[molten.node_control_loop.spec.tests] The implementation MUST include library or CLI coverage for bounded loop dispatch, duplicate idempotency, shutdown stop behavior, stale lock denial, and parseable CLI loop receipts.

#### Scenario: Loop tests pass
- GIVEN the Molten test suite
- WHEN node control loop tests run
- THEN bounded dispatch, idempotent duplicate, shutdown, stale lock, and CLI receipt paths are covered by canonical receipts.
