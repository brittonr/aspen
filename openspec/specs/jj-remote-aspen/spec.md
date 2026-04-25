# jj-remote-aspen Specification

## Purpose
TBD - created by archiving change jj-native-forge-wasm-plugin. Update Purpose after archive.
## Requirements
### Requirement: JJ clients can talk to Aspen natively

Aspen SHALL provide a JJ-native remote path for clone, fetch, push, and bookmark synchronization over iroh QUIC. JJ-native clients SHALL NOT be forced through `git-remote-aspen` or Git SHA-1 object translation.

#### Scenario: Native clone

- **WHEN** a JJ client clones a JJ-enabled Forge repository over Aspen transport
- **THEN** the client receives native JJ repo state from Forge
- **AND** the clone does not depend on the Git remote-helper protocol

#### Scenario: Native push

- **WHEN** a JJ client pushes new JJ commits and bookmark updates to Forge
- **THEN** Aspen sends JJ-native object and bookmark payloads to the JJ Forge backend
- **AND** the push response reports JJ-native success or conflict information

### Requirement: JJ-native access is exposed through a standalone helper

Aspen SHALL provide a standalone `jj-remote-aspen` helper that speaks the JJ-native Aspen wire protocol over iroh QUIC.

#### Scenario: Helper starts JJ-native session

- **WHEN** a JJ user invokes `jj-remote-aspen` for a JJ-enabled Forge repository
- **THEN** the helper performs JJ-native capability discovery and opens the JJ-native QUIC session using the advertised transport identifier
- **AND** the helper does not route the operation through `git-remote-aspen`

### Requirement: JJ remote operations enforce repo authorization

JJ-native clone, fetch, push, bookmark synchronization, and change-id resolution SHALL enforce the same repo-scoped authentication and authorization policy as other Forge operations.

#### Scenario: Authorized JJ read

- **WHEN** an authenticated client with read access performs JJ-native clone, fetch, or change-id resolution
- **THEN** Forge serves the requested JJ-native data
- **AND** the transport does not bypass repo read permissions

#### Scenario: Unauthorized JJ write

- **WHEN** a client without JJ write permission attempts a JJ-native push or bookmark update
- **THEN** Forge rejects the operation with an authorization error
- **AND** no JJ-visible repo state changes are published

### Requirement: JJ bookmark mutations use the native remote path

JJ bookmark create, move, and delete operations SHALL be available over the JJ-native remote path and SHALL return success, authorization, or conflict results explicitly.

#### Scenario: Move bookmark over JJ-native transport

- **WHEN** an authorized JJ client requests a bookmark move over the JJ-native remote path
- **THEN** Forge applies the bookmark update through the JJ backend
- **AND** the client receives JJ-native success or conflict information for that mutation

#### Scenario: Unauthorized bookmark mutation

- **WHEN** a client without JJ write permission attempts to create, move, or delete a bookmark over the JJ-native remote path
- **THEN** Forge rejects the request with an authorization error
- **AND** the bookmark state remains unchanged

### Requirement: JJ remote sync is incremental

The JJ-native remote path SHALL exchange only missing JJ objects and changed bookmark/change-index state whenever possible.

#### Scenario: Fetch only missing objects

- **WHEN** a JJ client already has most of a repo's JJ object graph locally
- **THEN** a fetch transfers only the missing JJ objects and updated bookmark/change-index state
- **AND** unchanged objects are not resent

#### Scenario: Bookmark-only sync

- **WHEN** no new JJ objects are required but bookmark positions changed on the server
- **THEN** the remote sync updates bookmark state without retransmitting unchanged JJ objects

#### Scenario: Push sends only missing objects

- **WHEN** a JJ client pushes a change graph and the server already has a subset of the referenced JJ objects
- **THEN** the client sends only the missing JJ objects plus the required bookmark/change updates
- **AND** already-present JJ objects are not resent unnecessarily

### Requirement: JJ transport version is explicit

The JJ-native wire protocol SHALL advertise an explicit transport version so clients and servers can reject incompatible protocol revisions before starting JJ object exchange.

#### Scenario: Compatible JJ transport version

- **WHEN** a JJ client and server advertise the same supported JJ transport version
- **THEN** JJ-native sync proceeds using that version

#### Scenario: Incompatible JJ transport version

- **WHEN** a JJ client targets a node that advertises JJ-native access for a different unsupported transport version
- **THEN** the session is rejected before JJ object exchange begins
- **AND** the client receives a compatibility error instead of a partial sync

### Requirement: JJ change IDs are queryable over the native remote path

The JJ-native remote path SHALL let clients resolve repo-scoped JJ change IDs to their current commit heads without falling back to Git metadata reconstruction.

#### Scenario: Resolve change ID over native transport

- **WHEN** a JJ client asks Forge to resolve a repo-scoped change ID over the JJ-native transport
- **THEN** Forge returns the current JJ commit head recorded for that change ID
- **AND** the response comes from JJ-native server state rather than Git SHA-1 translation

### Requirement: JJ-native transport uses a stable routing identifier

JJ-native sync SHALL use a stable network-visible QUIC routing identifier that is advertised during capability discovery and bound to the active JJ plugin registration on a node. This routing identifier SHALL be the manifest-declared protocol identifier / ALPN value for the JJ plugin, and it SHALL be the same transport identifier the node admits for JJ-native QUIC sessions.

#### Scenario: Discovery returns JJ routing identifier

- **WHEN** a client discovers JJ-native access for a repo on a capable node
- **THEN** the response includes the stable JJ routing identifier that the client must use for the next JJ-native session
- **AND** that identifier corresponds to the active JJ plugin registration on that node
- **AND** the node admits JJ-native QUIC sessions using that same identifier

#### Scenario: Plugin reload preserves routing identifier

- **WHEN** the active JJ plugin is reloaded or upgraded without changing its declared JJ transport identifier
- **THEN** subsequent discovery responses continue to advertise the same JJ routing identifier
- **AND** clients do not need a new transport identifier for the same JJ backend contract

#### Scenario: Inactive node does not advertise JJ routing identifier

- **WHEN** a target node does not currently have the JJ plugin active
- **THEN** Forge omits the JJ routing identifier for that node from discovery results
- **AND** the client is not asked to attempt a JJ-native session there

### Requirement: JJ remote support is discoverable

Forge SHALL expose whether a repository and target node support JJ-native access before a client begins a full sync.

#### Scenario: Repo advertises JJ support

- **WHEN** a JJ client queries repository capabilities for a JJ-enabled repo
- **THEN** Forge reports JJ-native access as supported for that repo
- **AND** the response includes enough routing information for the client to begin JJ-native sync

#### Scenario: Repo does not support JJ

- **WHEN** a JJ client targets a Git-only repository
- **THEN** Forge rejects JJ-native sync with a capability error
- **AND** the response does not silently fall back to Git transport

#### Scenario: Target node lacks active JJ plugin

- **WHEN** a JJ client queries JJ-native access for a JJ-enabled repo on a node that does not currently have the JJ plugin active
- **THEN** Forge reports JJ-native access as unavailable on that target node
- **AND** the response omits JJ routing information for that node instead of advertising a broken path

### Requirement: JJ remote streams remain bounded

JJ-native clone, fetch, and push flows SHALL use bounded streaming behavior so large repos do not create unbounded request or response stages.

#### Scenario: Large push uses chunked bounded transfer

- **WHEN** a JJ client pushes a large change graph that cannot fit in one small message
- **THEN** the transfer uses chunked or streamed exchange with explicit size and time bounds
- **AND** the server can reject the transfer cleanly when limits are exceeded

#### Scenario: Stream failure is retryable

- **WHEN** a JJ-native fetch or push stream fails before completion
- **THEN** the client receives a transport error instead of ambiguous success
- **AND** the next sync attempt can retry from consistent server state
