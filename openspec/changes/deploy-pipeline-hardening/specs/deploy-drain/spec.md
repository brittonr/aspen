## ADDED Requirements

### Requirement: Shared drain state between RPC handler and upgrade executor

The ClientProtocolContext SHALL carry an optional `Arc<DrainState>` that is shared between the client RPC handler and the NodeUpgradeExecutor. When handle_node_upgrade receives an upgrade request, it SHALL pass this shared drain state to the executor so the drain phase waits for real in-flight client RPCs to complete.

#### Scenario: DrainState present in context

- **WHEN** a node is bootstrapped with deploy support enabled
- **THEN** the ClientProtocolContext SHALL contain a non-None DrainState
- **AND** the same DrainState instance SHALL be used by both the RPC handler and the upgrade executor

#### Scenario: DrainState absent in context

- **WHEN** a node is bootstrapped without deploy support
- **THEN** the ClientProtocolContext DrainState SHALL be None
- **AND** the RPC handler SHALL skip drain checks and dispatch requests normally

### Requirement: Client RPCs rejected during drain

The ClientProtocolHandler SHALL check DrainState before dispatching each client RPC request. When the node is draining, new requests SHALL be rejected with a NOT_LEADER error so clients fail over to another node.

#### Scenario: RPC arrives before drain

- **WHEN** a client sends an RPC request and DrainState is not draining
- **THEN** the handler SHALL call try_start_op() and dispatch the request normally
- **AND** the handler SHALL call finish_op() after the request completes, including on error

#### Scenario: RPC arrives during drain

- **WHEN** a client sends an RPC request and DrainState is draining
- **THEN** try_start_op() SHALL return false
- **AND** the handler SHALL respond with NOT_LEADER error
- **AND** the handler SHALL NOT dispatch the request to any handler

#### Scenario: In-flight RPC completes during drain

- **WHEN** a drain starts while an RPC is in-flight (try_start_op returned true before drain)
- **THEN** the in-flight RPC SHALL complete normally
- **AND** finish_op() SHALL be called when it completes
- **AND** the drain SHALL wait for the in-flight count to reach zero

### Requirement: Drop guard for finish_op

The handler SHALL use a drop guard to ensure finish_op() is called even if the request handler panics or returns an error. The in-flight counter MUST NOT leak.

#### Scenario: Handler panics

- **WHEN** a request handler panics after try_start_op() returned true
- **THEN** the drop guard SHALL call finish_op()
- **AND** the in-flight count SHALL decrement correctly
