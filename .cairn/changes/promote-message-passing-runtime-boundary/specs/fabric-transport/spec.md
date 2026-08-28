# Fabric Transport Specification Delta

## ADDED Requirements

### Requirement: State-owner communication uses canonical messages

r[molten.message_boundary.contract]

Molten MUST use bounded, owned, versioned messages as the only semantic communication primitive across independent state-owner and external effect boundaries.

Each selected protocol contract MUST declare required identity, source, destination, generation, ordering, correlation, causation, payload, logical-time, deadline, authority, resource, and prior-operation fields plus finite bounds.

Pure calls and local mutation inside one state owner MAY remain direct.

#### Scenario: Adapter delivers an owned application message

- GIVEN a live adapter receives valid bounded transport bytes for an admitted protocol
- WHEN it crosses the application boundary
- THEN it MUST construct an owned canonical message with the required protocol fields
- AND the receiving state owner MUST apply its own transition policy.

#### Scenario: Local pure helper stays direct

- GIVEN a state transition calls a pure helper inside the same state owner
- WHEN no authority, mutable state, or external effect crosses the call
- THEN the implementation MUST NOT require an artificial transport message.

### Requirement: Live runtime handles stay in shells and adapters

r[molten.message_boundary.handle_containment]

Live sockets, Iroh connections, endpoints, clients, channels, senders, receivers, executors, tasks, borrowed transport buffers, and vendor session objects MUST NOT enter core state, canonical messages, callback values, application-owned port contracts, decisions, events, or effect plans.

Declared shell and adapter implementations MAY own these handles.

#### Scenario: Iroh connection remains private

- GIVEN an Iroh adapter owns a live connection and stream
- WHEN the adapter calls an application-owned port
- THEN the port MUST receive only owned handle-free application values
- AND no public accessor MUST expose the connection or stream handle.

#### Scenario: Nested wrapper leaks a handle

- GIVEN a core message or state field contains a live handle through an alias, generic wrapper, or associated type
- WHEN source and architecture admission run
- THEN admission MUST block and identify the resolved handle path.

### Requirement: Connection lifecycle enters semantics as messages

r[molten.message_boundary.connection_events]

Connection open, close, reset, retry, overload, cancellation, and uncertain delivery MAY affect application behavior only through canonical typed messages or explicit adapter diagnostics.

Logical session identifiers, stream identifiers, generations, finite phases, and delivery states MAY enter core values when they contain no live handle and derive from admitted messages.

#### Scenario: Stream-open callback is handle-free

- GIVEN a transport accepts a new logical stream
- WHEN the extension receives a stream-open callback
- THEN the callback MUST carry a canonical message with logical identifiers and declared bounds
- AND it MUST NOT carry the live stream or connection object.

#### Scenario: Disconnect follows an uncertain write

- GIVEN a frame can have reached the peer before the connection resets
- WHEN the adapter reports the outcome
- THEN it MUST emit uncertain delivery with the operation identity
- AND it MUST NOT infer that the application action did not commit.

### Requirement: Live and deterministic adapters preserve one message contract

r[molten.message_boundary.transport_parity]

Every live and deterministic adapter pair selected for message-boundary conformance MUST preserve the same application-owned message, event, lifecycle, identity, flow-control, cancellation, and failure contract.

Adapter-specific facts MUST remain explicit profile metadata or diagnostics and MUST NOT silently change base application meaning.

#### Scenario: Shared canonical trace matches

- GIVEN a bounded fixture whose live and deterministic profiles declare equivalent behavior
- WHEN both adapters execute the fixture through the same state-transition core
- THEN their normalized canonical application traces MUST match the declared allowed trace set.

#### Scenario: Live adapter exposes extra semantic state

- GIVEN the live adapter lets application code branch on a connection-only fact absent from the deterministic contract
- WHEN differential conformance runs
- THEN conformance MUST fail with an adapter-semantic-drift diagnostic.
