# Node Runtime Delta: Persistent Local Control Surface

### Requirement: Control inbox is persistent
r[molten.node_control_socket.spec.persistent_inbox] A Molten node control surface MUST persist submitted canonical `node-control-request-v1` values under the explicit node state root before dispatch.

#### Scenario: Submitted request is durable
- GIVEN an initialized node state root and a canonical status request
- WHEN the request is submitted to the local control profile
- THEN the request is written to the control inbox by request ref
- AND the same request value is importable from node ledger evidence.

### Requirement: Queue receipts bind request refs
r[molten.node_control_socket.spec.queue_receipts] The control surface MUST emit canonical queue or dispatch receipts that bind the request ref, operation, local profile, and decision.

#### Scenario: Queue receipt is canonical
- GIVEN a submitted request
- WHEN enqueue succeeds
- THEN a queue receipt is written with decision `pass`
- AND it binds the request ref and local Preserves control profile.

### Requirement: Ambient control roots are denied
r[molten.node_control_socket.spec.no_ambient_control] Control submission and dispatch MUST reject ambient state roots and MUST NOT infer the target node from the current working directory.

#### Scenario: Ambient root rejected
- GIVEN a control submit command with state root `.`
- WHEN validation runs
- THEN the command is denied before writing an inbox request.

### Requirement: Active process lock is explicit
r[molten.node_control_socket.spec.process_lock] A running node MUST write an active local control lock bound to startup evidence, and control dispatch MUST reject missing, stale, or duplicate lock state before side effects.

#### Scenario: Duplicate run lock denied
- GIVEN a node has an active startup lock without a clean shutdown
- WHEN another run or dispatch attempts to claim the same state root
- THEN the operation is denied before adapter or control side effects.

### Requirement: Dispatch decisions are receipt-backed
r[molten.node_control_socket.spec.dispatch_receipts] Control dispatch MUST produce canonical control receipts for status and shutdown requests and fail closed for unwired install, run, or gate operations before side effects.

#### Scenario: Submitted status passes
- GIVEN a running node and a submitted status request with authority and resource refs
- WHEN the request is dispatched
- THEN a `node-control-receipt-v1` with decision `pass` is written
- AND the receipt binds a health subreceipt.

#### Scenario: Unwired operation denies
- GIVEN a submitted install request
- WHEN the request is dispatched before the install adapter is wired
- THEN a deny receipt is emitted
- AND no install side effect occurs.

### Requirement: Authority and resource evidence gate control
r[molten.node_control_socket.spec.authority_resource_gate] Passing control receipts MUST bind explicit authority, policy, and resource evidence refs; missing evidence MUST produce a deny receipt.

#### Scenario: Missing authority denies
- GIVEN a status request without authority refs
- WHEN dispatch evaluates control admission
- THEN the control receipt decision is `deny`
- AND diagnostics identify missing authority evidence.

### Requirement: Control evidence is imported into the node ledger
r[molten.node_control_socket.spec.ledger_imports] The node control surface MUST import requests, queue receipts, health/shutdown/suboperation receipts, and final control receipts into the node ledger for later catalog and MCP inspection.

#### Scenario: Dispatch imports receipts
- GIVEN a dispatched status request
- WHEN ledger artifacts are listed under the node state root
- THEN node control request, queue receipt, health receipt, and control receipt artifacts are present.

### Requirement: Control socket tests cover lifecycle and denials
r[molten.node_control_socket.spec.tests] The control socket runtime MUST include library and CLI tests covering submit, status dispatch, shutdown dispatch, stale lock denial, ambient root denial, and unwired operation denial.

#### Scenario: CLI control lifecycle passes
- GIVEN the CLI integration suite
- WHEN it initializes a node, submits status and shutdown requests, and dispatches them
- THEN all canonical request and receipt artifacts are written and parseable.
