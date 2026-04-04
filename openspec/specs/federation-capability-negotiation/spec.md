## ADDED Requirements

### Requirement: Handshake returns actual peer capabilities

`connect_to_cluster()` SHALL return a `ConnectResult` containing the capabilities extracted from the peer's `FederationResponse::Handshake`. The capabilities SHALL NOT be hardcoded.

#### Scenario: Peer advertises forge and streaming-sync

- **WHEN** a peer responds to handshake with `capabilities: ["forge", "streaming-sync"]`
- **THEN** `ConnectResult.capabilities` SHALL equal `["forge", "streaming-sync"]`
- **AND** `ConnectResult.has_capability("forge")` SHALL return `true`
- **AND** `ConnectResult.has_capability("streaming-sync")` SHALL return `true`

#### Scenario: Peer advertises no capabilities (old peer)

- **WHEN** a peer responds to handshake with `capabilities: []` (or the field is absent due to older protocol)
- **THEN** `ConnectResult.capabilities` SHALL be empty
- **AND** `ConnectResult.has_capability("streaming-sync")` SHALL return `false`

#### Scenario: Unknown capability is preserved

- **WHEN** a peer responds with `capabilities: ["forge", "new-feature"]`
- **THEN** `ConnectResult.capabilities` SHALL contain `"new-feature"`
- **AND** `ConnectResult.has_capability("new-feature")` SHALL return `true`

### Requirement: Single connect function replaces dual API

`connect_to_cluster_full()` SHALL be removed. `connect_to_cluster()` SHALL be the sole connection entry point, returning `ConnectResult`.

#### Scenario: All callers use ConnectResult

- **WHEN** any code connects to a federated peer
- **THEN** it SHALL call `connect_to_cluster()` and receive a `ConnectResult`
- **AND** `connect_to_cluster_full` SHALL NOT exist in the codebase

### Requirement: Capability-gated operations

Callers SHALL check `ConnectResult::has_capability()` before attempting optional protocol features. If the peer lacks the required capability, the caller SHALL fall back to a compatible path or skip the operation.

#### Scenario: Streaming sync skipped for peer without capability

- **WHEN** a sync orchestrator connects to a peer
- **AND** `ConnectResult.has_capability("streaming-sync")` returns `false`
- **THEN** the orchestrator SHALL use batch sync instead of streaming sync

#### Scenario: Streaming sync used when capability present

- **WHEN** a sync orchestrator connects to a peer
- **AND** `ConnectResult.has_capability("streaming-sync")` returns `true`
- **THEN** the orchestrator SHALL use streaming sync
